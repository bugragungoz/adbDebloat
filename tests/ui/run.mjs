// The built interface in Chromium against the simulated phone (src/api/mock.ts): every page opens
// without an error, nothing changes before the review, protected apps cannot be picked, a batch
// shows each result and lands in History, undo restores, adb can be downloaded, a phone can be
// paired over Wi-Fi, and the smallest window has no sideways scroll.
//   npm run test:ui   (builds first; CHROMIUM_PATH picks a browser when Playwright finds none)
import assert from 'node:assert/strict';
import { after, before, test } from 'node:test';
import { launch } from './browser.mjs';
import { serve } from './server.mjs';

let server;
let browser;

before(async () => {
  server = await serve();
  browser = await launch();
});
after(async () => {
  await browser?.close();
  server?.close();
});

async function open({ scenario, width = 1280, height = 800, locale = 'en-US', colorScheme = 'dark' } = {}) {
  const context = await browser.newContext({ viewport: { width, height }, locale, colorScheme });
  const page = await context.newPage();
  const problems = [];
  page.on('pageerror', (e) => problems.push(e.message));
  page.on('console', (m) => {
    if (m.type() === 'error' || m.type() === 'warning') problems.push(m.text());
  });
  page.on('response', (r) => {
    if (r.status() >= 400) problems.push(`${r.status()} ${r.url()}`);
  });
  await page.goto(`${server.url}?fast${scenario ? `&scenario=${scenario}` : ''}`);
  await page.getByRole('heading', { name: /^(Phone|Telefon)$/ }).waitFor();
  return { page, context, problems };
}

const nav = (page, name) => page.getByRole('navigation').getByRole('button', { name: new RegExp(`^${name}`) }).click();
const row = (page, name) => page.getByRole('row', { name: new RegExp(name) });

test('every page opens without an error', async () => {
  const { page, context, problems } = await open();
  await page.getByRole('heading', { name: 'Redmi 5 Plus' }).waitFor();
  for (const [name, heading] of [
    ['Apps', 'Apps'],
    ['History', 'History'],
    ['Settings', 'Settings'],
    ['Phone', 'Phone'],
  ]) {
    await nav(page, name);
    await page.getByRole('heading', { name: heading, exact: true }).waitFor();
  }
  assert.deepEqual(problems, []);
  await context.close();
});

test('protected apps cannot be picked and say why', async () => {
  const { page, context } = await open();
  await nav(page, 'Apps');
  await row(page, 'MIUI System Ads|Weather').first().waitFor();
  // The launcher and the keyboard in use, and an Unsafe package, have no checkbox.
  for (const name of ['System Launcher', 'Gboard', 'System UI']) {
    const r = row(page, name);
    assert.equal(await r.getByRole('checkbox').count(), 0, name);
  }
  await row(page, 'Gboard').click();
  const panel = page.getByRole('complementary', { name: 'Gboard' });
  await panel.getByText('It is the keyboard in use').waitFor();
  assert.equal(await panel.getByRole('button', { name: 'Remove' }).count(), 0);
  // Expert ones too, until expert mode is on.
  await row(page, 'Battery & Performance').click();
  await page.getByRole('complementary', { name: 'Battery & Performance' }).getByText('Turn on expert mode').waitFor();
  await context.close();
});

test('nothing changes before the review, and the batch lands in History where it can be undone', async () => {
  const { page, context, problems } = await open();
  await nav(page, 'Apps');
  await row(page, 'Weather').first().waitFor();
  await page.getByRole('checkbox', { name: 'Select Weather' }).check();
  await page.getByRole('checkbox', { name: 'Select YouTube', exact: true }).check();
  await page.getByRole('checkbox', { name: 'Select Facebook Services' }).check();
  const bar = page.getByRole('region', { name: 'Selection' });
  await bar.getByText('3 selected').waitFor();

  // Cancel the review: nothing changed.
  await bar.getByRole('button', { name: 'Remove 3' }).click();
  let dialog = page.getByRole('dialog', { name: 'Remove 3 apps?' });
  await dialog.getByText('3 apps will be removed').waitFor();
  await dialog.getByRole('button', { name: 'Cancel' }).click();
  await dialog.waitFor({ state: 'detached' });
  assert.equal(await row(page, 'Select Weather').getByText('Installed', { exact: true }).count(), 1);

  // Disable: MIUI refuses it for its weather app, and the result says so.
  await bar.getByRole('button', { name: 'Disable 3' }).click();
  dialog = page.getByRole('dialog');
  await dialog.getByRole('button', { name: 'Disable 3 apps' }).click();
  await dialog.getByText('Done: 2 changed, 1 not changed.').waitFor();
  await dialog.getByText("The phone's maker does not let adb disable this app").waitFor();
  await dialog.getByText('Remove them instead').waitFor();
  await dialog.getByRole('button', { name: 'Close' }).last().click();

  // History lists the batch; Undo goes through the review again and restores both.
  await nav(page, 'History');
  await page.getByText('Disabled 3 apps').waitFor();
  await page.getByRole('button', { name: 'Undo' }).click();
  dialog = page.getByRole('dialog', { name: 'Undo this batch?' });
  await dialog.getByText('2 apps will be restored').waitFor();
  await dialog.getByRole('button', { name: 'Restore 2 apps' }).click();
  // The title follows the batch: "Restoring 2 apps", then "Finished".
  dialog = page.getByRole('dialog', { name: 'Finished' });
  await dialog.getByText('Done: 2 changed, 0 not changed.').waitFor();
  await dialog.getByRole('button', { name: 'Close' }).last().click();
  await page.getByText('Restored 2 apps').waitFor();
  assert.deepEqual(problems, []);
  await context.close();
});

test('select recommended picks only what may go', async () => {
  const { page, context } = await open();
  await nav(page, 'Apps');
  const select = page.getByRole('button', { name: /^Select \d+ recommended$/ });
  await select.waitFor();
  const n = Number((await select.innerText()).match(/\d+/)[0]);
  await select.click();
  await page.getByRole('region', { name: 'Selection' }).getByText(`${n} selected`).waitFor();
  await page.getByRole('region', { name: 'Selection' }).getByRole('button', { name: `Remove ${n}` }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByText(`${n} apps will be removed`).waitFor();
  // Every app in the list is marked Recommended, so there is no warning about others.
  assert.equal(await dialog.getByText('not marked Recommended').count(), 0);
  await context.close();
});

test('expert mode asks first and then lets Expert apps through', async () => {
  const { page, context } = await open();
  await nav(page, 'Settings');
  await page.getByRole('switch', { name: 'Expert mode' }).click();
  const dialog = page.getByRole('dialog', { name: 'Turn on expert mode?' });
  await dialog.getByRole('button', { name: 'Turn on' }).click();
  await nav(page, 'Apps');
  await row(page, 'Battery & Performance').getByRole('checkbox').waitFor();
  await context.close();
});

test('without adb, platform-tools can be downloaded', async () => {
  const { page, context, problems } = await open({ scenario: 'noadb' });
  await page.getByRole('heading', { name: 'adbDebloat needs adb' }).waitFor();
  await page.getByRole('button', { name: 'Download platform-tools' }).click();
  await page.getByText(/adb 35\.0\.2 is ready/).waitFor();
  await page.getByRole('heading', { name: 'Connect your phone' }).waitFor();
  assert.deepEqual(problems, []);
  await context.close();
});

test('a phone waiting for permission says what to do, then comes in', async () => {
  const { page, context } = await open({ scenario: 'unauthorized' });
  await page.getByText('Allow USB debugging on the phone').waitFor();
  await page.getByRole('heading', { name: 'Redmi 5 Plus' }).waitFor({ timeout: 8000 });
  await page.getByRole('button', { name: 'Choose apps to remove' }).waitFor();
  await context.close();
});

test('pairing and connecting over Wi-Fi', async () => {
  const { page, context } = await open({ scenario: 'nophone' });
  await page.getByRole('heading', { name: 'Connect your phone' }).waitFor();
  await page.getByLabel('Pairing address').fill('192.168.1.23:37145');
  await page.getByLabel('Pairing code').fill('000000');
  await page.getByRole('button', { name: 'Pair', exact: true }).click();
  await page.getByText(/Wrong password/).waitFor();
  await page.getByLabel('Pairing code').fill('123456');
  await page.getByRole('button', { name: 'Pair', exact: true }).click();
  await page.getByText(/^Paired\./).waitFor();
  await page.getByLabel('Address', { exact: true }).fill('192.168.1.23:41235');
  await page.getByRole('button', { name: 'Connect', exact: true }).click();
  await page.getByText('Connected over Wi-Fi', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Disconnect' }).waitFor();
  await context.close();
});

test('the language and theme change at once and are kept', async () => {
  const { page, context } = await open({ colorScheme: 'dark' });
  await nav(page, 'Settings');
  await page.getByRole('radio', { name: 'Light' }).click();
  assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), 'light');
  await page.getByRole('radio', { name: 'Türkçe' }).click();
  await page.getByRole('heading', { name: 'Ayarlar', exact: true }).waitFor();
  await page.reload();
  await page.getByRole('heading', { name: 'Telefon' }).waitFor();
  assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), 'light');
  assert.equal(await page.evaluate(() => document.documentElement.lang), 'tr');
  await context.close();
});

test('the smallest window has no sideways scroll, on every page', async () => {
  const { page, context, problems } = await open({ width: 960, height: 620, locale: 'tr-TR' });
  for (const name of ['Telefon', 'Uygulamalar', 'Geçmiş', 'Ayarlar']) {
    await nav(page, name);
    await page.waitForTimeout(150);
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    assert.equal(overflow, 0, `${name} scrolls sideways by ${overflow}px`);
  }
  assert.deepEqual(problems, []);
  await context.close();
});
