// The README screenshots, from the simulated phone (a made-up Redmi 5 Plus).
//   npm run shots   ->  docs/images/*.png
import { mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { launch } from './browser.mjs';
import { serve } from './server.mjs';

const OUT = fileURLToPath(new URL('../../docs/images/', import.meta.url));
await mkdir(OUT, { recursive: true });
const server = await serve();
const browser = await launch();

async function shoot(name, { colorScheme = 'dark', steps = async () => {} } = {}) {
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1, colorScheme, locale: 'en-US' });
  const page = await context.newPage();
  await page.goto(`${server.url}?fast`);
  await page.getByRole('heading', { name: 'Redmi 5 Plus' }).waitFor();
  await steps(page);
  await page.waitForTimeout(400);
  await page.screenshot({ path: `${OUT}${name}.png` });
  await context.close();
  console.log(`docs/images/${name}.png`);
}

const nav = (page, name) => page.getByRole('navigation').getByRole('button', { name: new RegExp(`^${name}`) }).click();

await shoot('phone');
await shoot('apps', {
  steps: async (page) => {
    await nav(page, 'Apps');
    await page.getByRole('checkbox', { name: 'Select MIUI Daemon' }).check();
    await page.getByRole('checkbox', { name: 'Select Quick Apps', exact: true }).check();
    await page.getByRole('checkbox', { name: 'Select Wallpaper Carousel' }).check();
    await page.getByRole('row', { name: /Wallpaper Carousel/ }).click();
  },
});
await shoot('review', {
  colorScheme: 'light',
  steps: async (page) => {
    await nav(page, 'Apps');
    await page.getByRole('button', { name: /^Select \d+ recommended$/ }).click();
    await page.getByRole('checkbox', { name: 'Select Gallery' }).check();
    await page.getByRole('region', { name: 'Selection' }).getByRole('button', { name: /^Remove/ }).click();
  },
});
await shoot('history', {
  colorScheme: 'light',
  steps: async (page) => {
    await nav(page, 'Apps');
    await page.getByRole('checkbox', { name: 'Select Weather' }).check();
    await page.getByRole('checkbox', { name: 'Select YouTube', exact: true }).check();
    await page.getByRole('checkbox', { name: 'Select Mi Video' }).check();
    await page.getByRole('region', { name: 'Selection' }).getByRole('button', { name: /^Disable/ }).click();
    await page.getByRole('dialog').getByRole('button', { name: /^Disable \d+ apps$/ }).click();
    await page.getByRole('dialog').getByText(/^Done:/).waitFor();
    await page.getByRole('dialog').getByRole('button', { name: 'Close' }).last().click();
    await nav(page, 'History');
    await page.getByRole('button', { name: 'Show apps' }).click();
  },
});

await browser.close();
server.close();
