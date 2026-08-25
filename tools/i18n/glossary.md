# Translation glossary

Binding decisions for `es` and `pt_BR`. Consistency across 660+ short strings is what makes a
translation feel native; ad-hoc choices are what make it feel machine-made. Consult this before
translating, and extend it when a new product term appears.

## Register

| Locale | Address the user as | Notes |
|---|---|---|
| `es` | **tú** (informal) | Neutral/international Spanish. Avoid `vos` and avoid `usted`. Prefer vocabulary shared across Spain and Latin America (`ordenador`/`computadora` → say `dispositivo`). |
| `pt_BR` | **você** (informal) | Brazilian Portuguese. Never European forms (`ecrã`, `telemóvel`, `utilizador`, `ficheiro`). |

Both match Decentraland's existing product tone: direct, brief, friendly. Prefer the imperative
for buttons (`Guardar`, `Salvar`) over the infinitive-as-noun.

## Terms that stay in English

These are product nouns and brand surfaces. Translating them breaks recognition against the
Marketplace, the docs and the wider community, which are English everywhere.

Wearable · Emote · Passport · Backpack · Jump In · MANA · Genesis City · Parcel · Scene ·
Realm · Marketplace · Decentraland · Smart Wearable · Snapshot · DAO · World · Skybox

Keep them capitalized as above and do **not** inflect them into Spanish/Portuguese plurals
beyond a bare `s` (`Wearables`, `Emotes`).

## Terms that are translated

| English | `es` | `pt_BR` |
|---|---|---|
| Settings | Ajustes | Configurações |
| Profile | Perfil | Perfil |
| Friends | Amigos | Amigos |
| Friend request | Solicitud de amistad | Pedido de amizade |
| Community | Comunidad | Comunidade |
| Nearby | Cerca | Por perto |
| Places | Lugares | Lugares |
| Events | Eventos | Eventos |
| Discover | Descubrir | Descobrir |
| Chat | Chat | Chat |
| Voice chat | Chat de voz | Chat de voz |
| Sign in / Sign out | Iniciar sesión / Cerrar sesión | Entrar / Sair |
| Wallet | Billetera | Carteira |
| Credits | Créditos | Créditos |
| Loading… | Cargando… | Carregando… |
| Retry | Reintentar | Tentar novamente |
| Skip | Omitir | Pular |
| Done | Listo | Pronto |
| Save | Guardar | Salvar |
| Cancel | Cancelar | Cancelar |
| Close | Cerrar | Fechar |
| Back | Atrás | Voltar |
| Next | Siguiente | Avançar |
| Owner / Creator | Propietario / Creador | Proprietário / Criador |
| Preview | Vista previa | Prévia |
| Report | Reportar | Denunciar |
| Block / Unblock | Bloquear / Desbloquear | Bloquear / Desbloquear |
| Mute / Unmute | Silenciar / Activar sonido | Silenciar / Ativar som |

## Mechanical rules

- **Placeholders are code, and always named.** `{name}` must survive verbatim. Reorder them
  freely to suit the target grammar — that is exactly why they are named, and why positional
  `%s` is rejected outright — but never rename, drop or invent one. `format_csv.py --check`
  fails the build on both. An unfilled field renders as literal `{braces}` on screen.
- **Padding and decimals are not the translator's problem.** `String.format()` has no format
  spec, so any such rendering happens in the calling code before substitution.
- **BBCode is markup.** `[b]`, `[color=#RRGGBB]`, `[/color]` pass through untouched; translate
  only the text between tags.
- **Never translate the key**, only the locale column.
- **Real newlines**, not `\n` — the importer runs with `unescape_translations=false`, so a
  literal backslash-n renders on screen. Embed the line break inside the quoted cell.
- **ALL-CAPS source stays ALL-CAPS** (button and section labels). Spanish and Portuguese
  uppercase keeps its accents: `SESIÓN`, `CONFIGURAÇÕES`.
- **Length.** ES/pt-BR run 20–30% longer than English. On buttons, tabs and chips choose the
  shorter natural wording rather than expecting the layout to absorb it.
- **Sentence case** for prose and labels; do not copy English title case.
- **Ellipsis**: match the source character (`...` vs `…`) rather than normalizing it.
