# Decentraland Marketplace Credits — Research & Proposal Notes

> Investigación end-to-end del sistema de Marketplace Credits de Decentraland, con foco en (a) cómo hacer que los credits no expiren y (b) cómo armar un endpoint de acreditación.
>
> **Objetivo declarado del usuario**: proponer cambios upstream al credits-server oficial de Decentraland (no fork ni deployment paralelo).

---

## TL;DR

- El backend oficial es **`decentraland/credits-server`** (privado). Tiene la `PRIVATE_KEY` que firma cada credit.
- Un credit existe como (1) row off-chain en `user_credits` y (2) struct firmado con `eth_sign` (NO EIP-712) que el contrato `CreditsManagerPolygon` verifica on-chain.
- La expiración se enforce en **3 capas**: contrato on-chain (vinculante), filtros SQL backend, frontend cosmético.
- Para credits no-expirables, **una "season perpetua" (end_date=2099) resuelve las 3 capas con cero cambios**, PERO requiere un flag `is_perpetual` y ~7 cambios de código para evitar 3 HARD blockers.
- El endpoint de acreditación oficial ya existe (`POST /users/:address/credits`) — la propuesta upstream sería agregar `expiresAt` opcional + concepto de issuer/source para partners.

---

## 1) Mapa de repositorios

| Repo | Rol |
|---|---|
| **`decentraland/credits-server`** (privado) | Backend Node/TS. Tiene `PRIVATE_KEY` que firma cada Credit. Expone API HTTP. |
| **`decentraland/credits-squid-core`** | Indexer Subsquid del contrato on-chain. Comparte Postgres con credits-server. |
| **`decentraland/offchain-marketplace-contract`** | Contiene `src/credits/CreditsManagerPolygon.sol`, contrato on-chain de redención. |
| **`decentraland/decentraland-transactions`** | Direcciones desplegadas + ABI del CreditsManager. |
| **`decentraland/decentraland-dapps`** | `CreditsClient` (llama al credits-server) + `CreditsService` (arma tx `useCredits` on-chain). |
| **`decentraland/marketplace`** | dApp; lee de `https://credits.decentraland.org` (config en `webapp/src/config/env/prod.json`). |

**No existe** `marketplace-credits-server` — el backend se llama solo `credits-server`.

### Direcciones desplegadas (`CreditsManagerPolygon`)
- Polygon mainnet (137): `0x8b3a40ca1b6f5cafc99d112a4d02e897d1fd8cc5`
- Polygon Amoy testnet (80002): `0x8052a560e6e6ac86eeb7e711a4497f639b322fb3`

---

## 2) Endpoints HTTP del credits-server

Definidos en **`src/controllers/routes.ts`**:

| Método | Path | Auth | Handler | Propósito |
|---|---|---|---|---|
| GET | `/status` | — | `get-service-status.ts` | Health |
| GET | `/users/:address/credits` | signed-fetch | `get-user-credits.ts` | Lista credits del user |
| GET | `/users/:address/credits/stream` | signed-fetch | `credits-sse.ts` | SSE balance updates |
| GET | `/users/:address/progress` | signed-fetch | `get-user-progress.ts` | Progreso de goals |
| GET | `/users/:address/status` | signed-fetch | `get-user-status.ts` | Estado del user |
| POST | `/users` | signed-fetch | `register-user.ts` | Enroll |
| DELETE | `/users/:address` | signed-fetch | `unregister-user.ts` | Unenroll |
| GET/POST | `/captcha` | signed-fetch | `{get,verify}-captcha.ts` | Anti-bot |
| POST | `/sign-external-call` | schema | `sign-external-call.ts` | Firma `ExternalCall` |
| GET | `/credits-name-route` | signed-fetch | `get-credits-name-route.ts` | CORAL para ENS |
| GET | `/season`, `/seasons`, `/v2/seasons` | — | `get-season-status.ts`, `get-seasons.ts` | Season info |
| **POST** | **`/users/:address/credits`** | bearer + signed-fetch + admin allowlist | **`grant-credits.ts`** | **Mintea Credit firmado (issuance)** |
| GET | `/credits` | bearer + signed-fetch + read-admin | `get-credits.ts` | Admin: listar todos |
| GET | `/credit-reasons` | bearer + read-admin | `get-credit-reasons.ts` | Admin: razones |
| POST | `/flag` / `/unflag` / etc. | bearer | `flag-users.ts` | Anti-fraud |

### El endpoint de issuance (triple auth)

```ts
// src/controllers/routes.ts:84-90
router.post(
  '/users/:address/credits',
  bearerTokenMiddleware(GRANTER_API_ADMIN_TOKEN),
  signedFetchMiddleware,
  readWriteAdmins.adminAuthMiddleware,
  grantCreditsHandler
)
```

Cap diario: `DAILY_GRANTER_LIMIT` (env). Throw `DailyGrantLimitExceededError` si se supera (`grant-credits.ts:124`).

---

## 3) Modelo de datos del Credit

Un Credit vive en **dos lugares**:

### 3a. Off-chain (Postgres `user_credits`)

| columna | significado |
|---|---|
| `id` (uuid, también `salt`) | unique |
| `user_address` | recipient |
| `amount` | wei |
| `contract` | address de `CreditsManagerPolygon` |
| `timestamp` | ms |
| `signature` | secp256k1 sig del `PRIVATE_KEY` del credits-server |
| `season_id` | FK → `seasons` |
| `goal_id`, `week_id` | grant tipo goal |
| `claimed_at` | nullable |
| **`expires_at`** | **bigint, unix-seconds, NOT NULL** |

`expires_at` agregado en migración `1747150767217_credit-add-expires-at.ts`.

### 3b. On-chain (struct firmado)

```solidity
// CreditsManagerPolygon.sol:143-150
struct Credit {
    uint256 value;
    uint256 expiresAt;  // <-- expiración aquí
    bytes32 salt;
}
// hash firmado:
keccak256(abi.encode(_sender, block.chainid, address(this), credit))
```

**Quirk crítico**: NO es EIP-712 — usa `signMessage` (prefijo `\x19Ethereum Signed Message:\n`) y el contrato recovera con `creditHash.toEthSignedMessageHash().recover(...)` (línea 770).

**El contrato NO almacena credits**. Solo guarda:
- `mapping(bytes32 => uint256) public spentValue;`
- `mapping(bytes32 => bool) public isRevoked;`

Replay protection: `salt = keccak256(uuid)` dentro del hash.

---

## 4) Issuance flow (cómo se reciben credits)

Dos entrypoints, ambos pasan por `createSignedCredit()`:

1. **Goal-based** (regular): worker observa actividad (visit-scene, visit-event, log-in, take-photo, visit-profile en `src/logic/goals-handlers/`) → `creditsGranter.grantGoalCredits(addr, goalId)` cuando se completa.
2. **On-demand** (admin): `POST /users/:address/credits` con `{ type:'on-demand', amount, reason, annotation? }` → `creditsGranter.grantOnDemandCredits(...)`.

Ambos convergen en `createSignedCredit()` (`src/logic/credits-granter.ts:46-115`):

1. Lookup current `Season`.
2. `expiresAt = season.endDate + CREDITS_GRACE_PERIOD_DAYS` (default 14d).
3. UUID random; `salt = keccak256(toUtf8Bytes(uuid))`.
4. `getContract(ContractName.CreditsManager, CHAIN_ID)`.
5. `signer.sign()` — único uso de `PRIVATE_KEY`.
6. INSERT `user_credits`.
7. Return `{ creditId, signature, expiresAt, season, seasonId }`.

**El "gate" para emitir credits es**: tener una private key cuyo address tenga `CREDITS_SIGNER_ROLE` en `CreditsManagerPolygon`. Nada más es enforced on-chain.

---

## 5) Redemption flow (cómo se gastan)

`decentraland-dapps/src/lib/credits.ts` exporta `CreditsService` con 5 sabores:

- `useCreditsCollectionStore` — primary sales wearables/emotes
- `useCreditsMarketplace` — secondary off-chain trades
- `useCreditsLegacyMarketplace` — old order book
- `useCreditsCollectionManager` — publicar collections
- `useCreditsWithExternalCall` — genérico (CORAL cross-chain ENS)

Cada uno arma `UseCreditsArgs { credits, creditsSignatures, externalCall, customExternalCallSignature, maxUncreditedValue, maxCreditedValue }` → `CreditsManagerPolygon.useCredits(...)`.

On-chain (`CreditsManagerPolygon.sol:750-804`), per credit:
1. `creditHash = keccak256(abi.encode(_sender, chainid, address(this), credit))`
2. `if (block.timestamp >= credit.expiresAt) revert CreditExpired(creditHash);` ← **expiración**
3. `if (isRevoked[creditHash]) revert RevokedCredit(...);`
4. `recovered = creditHash.toEthSignedMessageHash().recover(creditsSignatures[i])`
5. `if (!hasRole(CREDITS_SIGNER_ROLE, recovered)) revert InvalidSignature(...);`
6. Update `spentValue[creditHash]`, emit `CreditUsed`.

Después: external call (ej. `Marketplace.accept(...)`) y reconciliación de MANA.

---

## 6) Las 3 capas de expiración

| Capa | Dónde | Vinculante |
|---|---|---|
| **A. On-chain** | `CreditsManagerPolygon.sol:760` → `if (block.timestamp >= credit.expiresAt) revert CreditExpired(...)` | **SÍ** — `expiresAt` está en payload firmado, no se modifica post-firma |
| **B. Backend SQL** | `db.ts:56` y `:1138` filtran `WHERE uc.expires_at > now()`. También `db.ts:200-203` filtra `seasons.end_date >= now()` | Solo oculta de la API |
| **C. Frontend** | Enum `CreditStatus.EXPIRED` definido pero nunca emitido (backend filtra) | No vinculante |

### Para que NO expiren — opciones

#### Opción quick & dirty (suficiente a nivel contrato)
`expiresAt = type(uint256).max` al firmar. Pasa el chequeo on-chain. Pero hay que también:
- Quitar/relajar filtro `WHERE expires_at > now()` (`db.ts:56`, `:1138`)
- Quitar/relajar filtro `seasons.end_date >= now()` (`db.ts:200-203`)

**Limitación**: credits ya emitidos NO se pueden extender (su `expiresAt` está baked en la firma).

#### Opción "season perpetua" (más limpia con flag)
Insertar row en `seasons` con `end_date = 2099-12-31`. El granter calcula `expires_at = season.endDate + grace` automáticamente → 2099. Resuelve A, B (y C derivada) en una sola pieza.

**Pero**: requiere flag `is_perpetual` para evitar 3 HARD blockers (ver §8).

---

## 7) Auth model para crediting endpoint propio

### Gate 1 — On-chain (única que enforce el contrato)

```solidity
bytes32 public constant CREDITS_SIGNER_ROLE = keccak256("CREDITS_SIGNER_ROLE");
if (!hasRole(CREDITS_SIGNER_ROLE, recoveredSigner)) revert InvalidSignature(...);
```
(`CreditsManagerPolygon.sol:26`, `:772-775`)

Rate-limit on-chain: `maxManaCreditedPerHour` (líneas 70-76, 811-817).

Solo **un signer allowlisted** por deployment (puede haber más vía `grantRole`, llamable solo por `DEFAULT_ADMIN_ROLE`).

### Gate 2 — Off-chain (solo si llamás a la API de DCL)
- `bearerTokenMiddleware(GRANTER_API_ADMIN_TOKEN)`
- `signedFetchMiddleware` (signed-fetch del granter, recovered address = `granterAddress`)
- `readWriteAdmins.adminAuthMiddleware` (allowlist en `READ_WRITE_ADMIN_ADDRESSES` env)
- Cap diario `DAILY_GRANTER_LIMIT`

### Caminos para tener endpoint propio

**Camino A — Tu propio CreditsManager (autonomía total)**
1. Deploy de copia de `CreditsManagerPolygon.sol` con tu `creditsSigner` constructor param.
2. Reimplementar `signer.ts:46-88` (~40 líneas).
3. Fundear con MANA.
4. **Pero**: credits no aparecen en Marketplace de DCL — necesitás UI/checkout propios.

**Camino B — Push upstream a DCL (lo que el usuario quiere)**
- Requiere acuerdo con DCL.
- O bien que ejecuten `grantRole(CREDITS_SIGNER_ROLE, tuAddress)`, o que te agreguen a `READ_WRITE_ADMIN_ADDRESSES`.
- Y/o que mergen los cambios en credits-server para soportar:
  - `expiresAt` opcional / `is_perpetual` flag
  - Concepto de issuer/source para partners
  - Cuotas separadas por issuer

---

## 8) Análisis de riesgo: "season perpetua" (end_date=2099)

### El núcleo del problema

`db.getCurrentSeason()` (`src/adapters/db/db.ts:355-373`) es la única consulta de "current season":

```sql
SELECT ... FROM seasons
WHERE start_date <= now() AND end_date >= now()
ORDER BY start_date DESC LIMIT 1
```

El `ORDER BY start_date DESC LIMIT 1` no tira excepción, pero puede elegir silenciosamente la perpetua. Eso cascadea bombas.

### HARD blockers (rompen el sistema sin mitigación)

**1. Goal-completions tiran errores constantes.** `creditsGranter.grantGoalCredits` busca `getSeasonGoals(currentSeasonId, weekNumber)`. Si la perpetua gana, no tiene rows en `season_goals` → `Error('Goal not found for season and week')` cada vez que un usuario completa goal. Caught upstream pero genera ruido constante en logs.

**2. Dual-grant silencioso.** Constraint partial unique de `user_credits` (`migration 1759168116982`) es `(user_address, season_id, goal_id, week_id)` **scopeada por `season_id`**. Mismo usuario puede completar mismo goal dos veces — una bajo season regular, otra bajo perpetua — sin violar constraint. Doble crédito.

**3. `maxMana=0` flipea estado de season.** `season-service.ts:259` (`getSeasonState`): `if (totalGranted >= season.maxMana) return ERR_SEASON_RUN_OUT_OF_FUNDS`. Si `max_mana=0` en la perpetua, primer grant la marca como sin fondos → `/season` muestra "ran out of funds" a todos.

### SOFT issues (cosmético / operacional)

- `/season` y `/seasons` devuelven perpetua → frontend renderiza "expires in 73 years", `weekNumber=3000+`.
- `getCreditsExpiresIn` devuelve ~2.3e9 segundos.
- `reminder-email-sender` nunca dispara para credits perpetuos (probablemente *deseado*).
- Métricas Prom (`updateManaMetrics`) crecen monótonamente por años.
- `restart-user-progress` solo resetea "current season" — si gana perpetua, resetea solo perpetua.
- `getNextSeason` puede devolver perpetua si `start_date` futura → enmascara próxima season real.

### NON-issues (parecía sospechoso pero está OK)

- **Aritmética / overflow**: end_date 2099 = ~4.1e12 ms, dentro de `Number.MAX_SAFE_INTEGER`. `expires_at` en DB es `bigint`. Contrato usa `uint256`. Sin overflow.
- **`DAILY_GRANTER_LIMIT`**: filtra por `goal_id='on-demand-granted-credits'` y timestamp, NO por `season_id`. Perpetua no lo bypassa.
- **Admin `/credits`**: no filtra por season activa, ve todo. OK.
- **Schema constraints**: no hay UNIQUE/EXCLUDE que prevenga inserts overlapping. Coexisten sin error a nivel DB.

### Recaudo operacional crítico

Una vez creada la row, **NO se puede borrar**: hay FKs con RESTRICT desde `user_credits.season_id` y `rollbacked_credits.season_id`. Si en el futuro quisieran "deprecar" la season perpetua, primero migrar todos esos rows.

---

## 9) Forma propuesta del PR upstream

### Schema migration

```sql
ALTER TABLE seasons ADD COLUMN is_perpetual BOOLEAN NOT NULL DEFAULT false;

-- (opcional, recomendado): prevenir overlap entre seasons regulares
ALTER TABLE seasons ADD CONSTRAINT no_overlap_regular
  EXCLUDE USING gist (tsrange(start_date, end_date) WITH &&)
  WHERE (NOT is_perpetual);
```

### Cambios de código (mínimos, backwards-compat total)

1. **`db.getCurrentSeason()`**: agregar `AND is_perpetual = false`. Esto solo arregla los 3 HARD blockers (la perpetua deja de robarse goal-grants, weeks, métricas, state-flips).
2. **`db.getNextSeason()` / `getLastCompletedSeason()`**: idem.
3. **`getPastCurrentAndNextSeasons`** (`/seasons`): excluir perpetua de current/next/last; opcionalmente exponer como campo separado `perpetualSeason`.
4. **`reminder-email-sender`**: short-circuit si season es perpetua.
5. **`updateManaMetrics`**: skip o etiquetar `kind=perpetual` para que dashboards filtren.
6. **`getSeasonState`**: para perpetuas, omitir chequeo de `maxMana` o requerir `max_mana` muy alto.
7. **Granter**: agregar `grantOnDemandCreditsToPerpetualSeason()` o similar que lookup la perpetua por flag explícitamente.

### Endpoint de acreditación (extensión de `POST /users/:address/credits`)

Body actual:
```json
{ "type": "on-demand", "amount": "...", "reason": "...", "annotation": "..." }
```

Body propuesto (backwards-compat):
```json
{
  "type": "on-demand",
  "amount": "...",
  "reason": "...",
  "annotation": "...",
  "expiresAt": 9999999999,        // OPCIONAL — override del cálculo automático
  "perpetual": true,               // OPCIONAL — usa season perpetua, expiresAt=type(uint256).max
  "issuer": "regenesis-iap"        // OPCIONAL — para distinguir partner grants (cuotas separadas)
}
```

### Narrativa de venta para DCL

NO es "hackeo una row a 2099". ES "introduzco first-class concept de `is_perpetual` season para soportar credits sin expiración (eventos, partner programs, gift cards, IAP rewards), con backwards-compat total y mitigaciones explícitas en cada punto del sistema."

---

## 10) Key file references (cheat sheet)

### credits-server (privado)
- `src/controllers/routes.ts:84-90` — endpoint POST con triple auth
- `src/controllers/handlers/grant-credits.ts` — handler issuance
- `src/logic/credits-granter.ts:46-115` — `createSignedCredit()`
- `src/logic/credits-granter.ts:60-67` — cálculo de `expiresAt`
- `src/logic/signer.ts:15-88` — uso de `PRIVATE_KEY`
- `src/logic/season-service.ts:80-106` — `getCurrentSeasonAndWeek()`
- `src/logic/season-service.ts:240-280` — `getSeasonState()` (mana cap)
- `src/logic/admin/component.ts:18-34` — admin allowlist
- `src/logic/utils.ts` — `getCreditsExpirationTimeAfterSeasonEnd`
- `src/adapters/db/db.ts:52-60` — `getUserCreditsWhereClause` (expires_at filter)
- `src/adapters/db/db.ts:200-203` — LEFT JOIN active-season filter
- `src/adapters/db/db.ts:285-352` — `createOnDemandCredit` (DAILY_GRANTER_LIMIT global)
- `src/adapters/db/db.ts:355-373` — **`getCurrentSeason()` (smoking gun)**
- `src/adapters/db/db.ts:567-576`, `:828-839` — agregaciones por season
- `src/adapters/db/db.ts:1133-1144` — `getUserCreditsMaxExpiresAt`
- `src/migrations/1747150767217_credit-add-expires-at.ts` — adds `expires_at`
- `src/migrations/1755098828222_user-credits-constraint.ts` & `1759168116982_update-credits-constraint.ts` — partial unique índice por `season_id`

### offchain-marketplace-contract (público)
- `src/credits/CreditsManagerPolygon.sol:26` — `CREDITS_SIGNER_ROLE`
- `src/credits/CreditsManagerPolygon.sol:143-150` — struct `Credit`
- `src/credits/CreditsManagerPolygon.sol:750-804` — `useCredits` core loop
- `src/credits/CreditsManagerPolygon.sol:759-775` — expiration + signature checks
- `script/DeployCreditsManagerPolygon.s.sol` — role wiring at deploy

### decentraland-dapps (público)
- `src/modules/credits/CreditsClient.ts` — fetch al credits-server
- `src/lib/credits.ts:102-162` — `CreditsService` (5 useCredits flavors)

### marketplace (público)
- `webapp/src/config/env/prod.json` — `CREDITS_SERVER_URL=https://credits.decentraland.org`

---

## 11) Open questions / next steps

1. **Caso de uso concreto** que justifique credits perpetuos (gift cards, IAP rewards, partner programs, evento) → define cómo plantear la propuesta y si necesita "third-party issuer" con cap propio.
2. **Validar con DCL** antes de escribir PR: ¿están abiertos al concepto? ¿prefieren "perpetual season" vs `expiresAt` opcional explícito vs combo?
3. **Auth model para partner crediting**: ¿allowlist on-chain (más signers con `CREDITS_SIGNER_ROLE`)? ¿allowlist off-chain (más entries en `READ_WRITE_ADMIN_ADDRESSES`)? ¿ambos?
4. **Cuotas por issuer**: si DCL acepta partners, probablemente quieran un cap por-issuer en vez de global `DAILY_GRANTER_LIMIT`.
5. **Verificar** comportamiento exacto del UI del marketplace con `expires_at` muy lejano — ¿muestra "Dec 14, 2099" o algún sentinel "no expira"?
6. **Confirmar acceso al repo privado** `credits-server` antes de proponer PR.

---

## 12) Background — sources

- [Marketplace Credits launch blog](https://decentraland.org/blog/announcements/marketplace-credits-earn-weekly-rewards-to-power-up-your-look)
- [Marketplace Credits Season 3](https://decentraland.org/blog/announcements/marketplace-credits-season-3-keep-exploring-keep-earning)
- [Decentraland Rewards Terms (expiration policy)](https://decentraland.org/rewards-terms/)
- [Earning Rewards docs](https://docs.decentraland.org/in-world/earning-rewards)
- [Marketplace Credits landing](https://marketplace-credits.decentraland.org/)
- [credits-server README](https://github.com/decentraland/credits-server/blob/main/README.md)
- [CreditsManagerPolygon.sol](https://github.com/decentraland/offchain-marketplace-contract/blob/main/src/credits/CreditsManagerPolygon.sol)
- [decentraland-transactions creditsManager addresses](https://github.com/decentraland/decentraland-transactions/blob/main/src/contracts/creditsManager.ts)
