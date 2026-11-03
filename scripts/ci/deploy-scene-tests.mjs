#!/usr/bin/env node
//
// Deploys the scene tests to the QA World as production builds, where each scene shows its manual
// QA steps (docs/scene-tests.md); or a PR's changed scenes into that PR's zone; or removes a zone.
// CI runs it (scene_tests_world.yml): the World comes from SCENE_TESTS_WORLD and the deployer's key
// only from the DCL_PRIVATE_KEY environment variable, which that workflow fills from a secret.
//
//   deploy-scene-tests.mjs deploy [--zone <pr>] [--base origin/main]
//   deploy-scene-tests.mjs undeploy --zone <pr>
//   SCENE_TESTS_WORLD=mobile1.dcl.eth deploy-scene-tests.mjs deploy [--zone <pr>] --dry-run
//       what would be deployed where, with its links; needs no key, so it runs anywhere
import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import { createRequire } from 'node:module'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
const SUITE = path.join(ROOT, 'tests/scene-tests')
const SDK_COMMANDS = path.join(SUITE, 'node_modules/.bin/sdk-commands')
const WORLDS = (process.env.WORLDS_SERVER || 'https://worlds-content-server.decentraland.org').replace(/\/+$/, '')

// A PR's zone: 20x20 parcels on the World's west half (x -150..-31), away from the merged scenes'
// own parcels. Two PRs share a zone only when their numbers are 90 apart.
const ZONE_SIZE = 20
const ZONE_COLUMNS = 6
const ZONE_ROWS = 15

const require = createRequire(path.join(SUITE, 'package.json'))
const { Authenticator } = require('@dcl/crypto')
const { ethSign, recoverAddressFromEthSignature } = require('@dcl/crypto/dist/crypto')

function fail(message) {
  console.error(message)
  process.exit(1)
}

function parseArgs(argv) {
  const [command, ...rest] = argv
  const options = { command, zone: null, base: 'origin/main', dryRun: false }
  for (let i = 0; i < rest.length; i += 1) {
    if (rest[i] === '--zone') options.zone = Number(rest[++i])
    else if (rest[i] === '--base') options.base = rest[++i]
    else if (rest[i] === '--dry-run') options.dryRun = true
    else fail(`unknown argument: ${rest[i]}`)
  }
  if (!['deploy', 'undeploy'].includes(command)) fail('usage: deploy-scene-tests.mjs deploy|undeploy [--zone <pr>] [--base <ref>]')
  if (options.zone !== null && !Number.isInteger(options.zone)) fail('--zone takes a PR number')
  if (command === 'undeploy' && options.zone === null) fail('undeploy needs --zone <pr>')
  return options
}

function zoneOf(pr) {
  const index = pr % (ZONE_COLUMNS * ZONE_ROWS)
  return { x: -150 + (index % ZONE_COLUMNS) * ZONE_SIZE, y: -150 + Math.floor(index / ZONE_COLUMNS) * ZONE_SIZE }
}

const parse = (parcel) => parcel.split(',').map(Number)

/** Every scene folder (`x,y-name`), with its parcels from scene.json. */
function sceneFolders() {
  return fs
    .readdirSync(SUITE)
    .filter((name) => /^-?\d+,-?\d+-/.test(name) && fs.existsSync(path.join(SUITE, name, 'scene.json')))
    .map((folder) => ({ folder, scene: JSON.parse(fs.readFileSync(path.join(SUITE, folder, 'scene.json'), 'utf8')).scene }))
}

/** The scene folders a PR changed; every scene when it changed what they all share. */
function changedFolders(base, all) {
  const changed = execFileSync('git', ['diff', '--name-only', `${base}...HEAD`, '--', 'tests/scene-tests'], {
    cwd: ROOT,
    encoding: 'utf8'
  })
    .split('\n')
    .filter(Boolean)
    .map((file) => file.split('/')[2])
  if (changed.some((top) => top === 'testing-library' || top === 'package-lock.json' || top === 'package.json')) return all
  return all.filter(({ folder }) => changed.includes(folder))
}

/** Moves the scenes, keeping their layout, so their lowest corner lands on the zone's. */
function placeInZone(scenes, zone) {
  const parcels = scenes.flatMap(({ scene }) => scene.parcels.map(parse))
  const minX = Math.min(...parcels.map(([x]) => x))
  const minY = Math.min(...parcels.map(([, y]) => y))
  const maxX = Math.max(...parcels.map(([x]) => x))
  const maxY = Math.max(...parcels.map(([, y]) => y))
  if (maxX - minX >= ZONE_SIZE || maxY - minY >= ZONE_SIZE) fail(`the changed scenes span more than ${ZONE_SIZE}x${ZONE_SIZE} parcels`)
  const move = (parcel) => {
    const [x, y] = parse(parcel)
    return `${x - minX + zone.x},${y - minY + zone.y}`
  }
  return scenes.map(({ folder, scene }) => ({ folder, parcels: scene.parcels.map(move), base: move(scene.base) }))
}

function deployScene(world, key, { folder, parcels, base }) {
  const sceneJsonPath = path.join(SUITE, folder, 'scene.json')
  const original = fs.readFileSync(sceneJsonPath, 'utf8')
  const sceneJson = JSON.parse(original)
  sceneJson.worldConfiguration = { name: world }
  sceneJson.scene = { ...sceneJson.scene, parcels, base }
  fs.writeFileSync(sceneJsonPath, JSON.stringify(sceneJson, null, 2))
  const cwd = path.join(SUITE, folder)
  try {
    execFileSync(SDK_COMMANDS, ['build', '--production', '--skip-install'], { cwd, stdio: 'inherit' })
    execFileSync(
      SDK_COMMANDS,
      ['deploy', '--target-content', WORLDS, '--multi-scene', '--skip-build', '--no-browser'],
      { cwd, stdio: 'inherit', env: { ...process.env, DCL_PRIVATE_KEY: key } }
    )
  } finally {
    fs.writeFileSync(sceneJsonPath, original)
  }
}

/** ADR-44 signed fetch as the deployer, for the Worlds calls a deploy does not cover. */
async function signedFetch(key, url, method) {
  const pk = Uint8Array.from(Buffer.from(key.replace(/^0x/, ''), 'hex'))
  const address = recoverAddressFromEthSignature(ethSign(pk, 'address'), 'address')
  const timestamp = String(Date.now())
  const metadata = JSON.stringify({ signer: 'scene-tests-ci' })
  const payload = [method, new URL(url).pathname, timestamp, metadata].join(':').toLowerCase()
  const chain = Authenticator.createSimpleAuthChain(payload, address, ethSign(pk, payload))
  const headers = { 'x-identity-timestamp': timestamp, 'x-identity-metadata': metadata }
  chain.forEach((link, index) => {
    headers[`x-identity-auth-chain-${index}`] = JSON.stringify(link)
  })
  return fetch(url, { method, headers })
}

async function worldScenes(world) {
  const scenes = []
  for (let offset = 0; ; offset += 100) {
    const response = await fetch(`${WORLDS}/world/${encodeURIComponent(world)}/scenes?limit=100&offset=${offset}`)
    if (!response.ok) fail(`Worlds answered ${response.status} listing ${world}'s scenes`)
    const page = (await response.json()).scenes ?? []
    scenes.push(...page)
    if (page.length < 100) return scenes
  }
}

async function undeployZone(world, key, zone) {
  const inZone = (parcel) => {
    const [x, y] = parse(parcel)
    return x >= zone.x && x < zone.x + ZONE_SIZE && y >= zone.y && y < zone.y + ZONE_SIZE
  }
  for (const scene of await worldScenes(world)) {
    const parcel = (scene.parcels ?? []).find(inZone)
    if (parcel === undefined) continue
    const url = `${WORLDS}/world/${encodeURIComponent(world)}/scenes/${encodeURIComponent(parcel)}`
    const response = await signedFetch(key, url, 'DELETE')
    // already gone is what we wanted
    if (!response.ok && response.status !== 404) fail(`Worlds refused to undeploy ${parcel}: ${response.status} ${await response.text()}`)
    console.log(`Undeployed the scene on ${parcel}`)
  }
}

function report(world, deployed, zone) {
  const lines = [
    zone === null ? `### QA World: ${world}` : `### QA zone ${zone.x},${zone.y} in ${world}`,
    '',
    '| Scene | Position | Open |',
    '|---|---|---|'
  ]
  for (const { folder, base } of deployed) {
    const query = `realm=${encodeURIComponent(world)}&position=${base}`
    lines.push(`| ${folder} | ${base} | [mobile](https://mobile.dclexplorer.com/open?${query}) · [desktop](https://decentraland.org/play/?${query}) |`)
  }
  const text = lines.join('\n')
  console.log(text)
  if (process.env.GITHUB_STEP_SUMMARY) fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${text}\n`)
}

const options = parseArgs(process.argv.slice(2))
const world = (process.env.SCENE_TESTS_WORLD || '').trim().toLowerCase()
const key = (process.env.DCL_PRIVATE_KEY || '').trim()
if (!world) fail('SCENE_TESTS_WORLD is not set (the QA World, e.g. name.dcl.eth)')
if (!key && !options.dryRun) fail('DCL_PRIVATE_KEY is not set (the deployer wallet the World owner allowed)')
const zone = options.zone === null ? null : zoneOf(options.zone)

if (options.command === 'undeploy') {
  await undeployZone(world, key, zone)
} else {
  const all = sceneFolders()
  const picked = zone === null ? all : changedFolders(options.base, all)
  if (picked.length === 0) {
    console.log('No scene test changed: nothing to deploy.')
    process.exit(0)
  }
  const targets = zone === null ? picked.map(({ folder, scene }) => ({ folder, parcels: scene.parcels, base: scene.base })) : placeInZone(picked, zone)
  // One at a time: Worlds holds at most 10 pending deployments per deployer.
  if (!options.dryRun) for (const target of targets) deployScene(world, key, target)
  report(world, targets, zone)
}
