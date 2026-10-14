const assert = require('node:assert/strict');
const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const { chromium } = require('playwright');

const root = path.resolve(process.env.WEB_ROOT || 'web');
const mime = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json' };
const server = http.createServer((request, response) => {
  const pathname = new URL(request.url, 'http://localhost').pathname;
  const file = path.resolve(root, `.${pathname.endsWith('/') ? `${pathname}index.html` : pathname}`);
  if (!file.startsWith(`${root}${path.sep}`) || !fs.existsSync(file) || !fs.statSync(file).isFile()) {
    response.writeHead(404); response.end(); return;
  }
  response.setHeader('Content-Type', mime[path.extname(file)] || 'application/octet-stream');
  response.end(fs.readFileSync(file));
});

(async () => {
  let browser;
  try {
    await new Promise((resolve, reject) => {
      server.once('error', reject);
      server.listen(0, '127.0.0.1', resolve);
    });
    browser = await chromium.launch({
      headless: true,
      executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE || undefined,
      args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--enable-webgl', '--ignore-gpu-blocklist'],
    });
    const page = await browser.newPage({ viewport: { width: 1100, height: 1000 } });
    page.setDefaultTimeout(30000);
    const errors = [];
    page.on('pageerror', error => {
      if (!error.message.includes('Using exceptions for control flow')) errors.push(error.message);
    });
    // A fixed entropy seed makes the same game reproducible across browser runs.
    await page.addInitScript(() => {
      Object.defineProperty(Crypto.prototype, 'getRandomValues', {
        value(array) {
          const bytes = new Uint8Array(array.buffer, array.byteOffset, array.byteLength);
          for (let i = 0; i < bytes.length; i++) bytes[i] = (i * 17 + 3) & 255;
          return array;
        },
      });
    });
    const base = `http://127.0.0.1:${server.address().port}`;
    const response = await page.request.get(`${base}/pkg/rordle_bg.wasm`);
    assert.equal(response.headers()['content-type'], 'application/wasm');
    assert.equal(WebAssembly.validate(await response.body()), true, 'compiled WASM must validate');
    await page.goto(base);
    await page.waitForSelector('#canvas[data-ready="1"]');
    assert.equal(await page.locator('#keyboard button:disabled').count(), 0);
    const canvas = page.locator('#canvas');
    const settle = () => page.waitForTimeout(300);
    await settle();
    const initial = await canvas.screenshot();
    await page.keyboard.type('apple');
    await settle();
    assert.equal((await canvas.screenshot()).equals(initial), false, 'typing must change the board');
    for (let i = 0; i < 5; i++) await page.keyboard.press('Backspace');
    await settle();
    assert.equal((await canvas.screenshot()).equals(initial), true, 'backspace must clear input');
    await page.locator('#keyboard button[data-key="a"]').click();
    await settle();
    assert.equal((await canvas.screenshot()).equals(initial), false, 'touch keyboard must enter a letter');
    await page.locator('#keyboard button[data-key="Backspace"]').click();
    await settle();
    assert.equal((await canvas.screenshot()).equals(initial), true, 'touch backspace must clear input');
    await canvas.click({ position: { x: 130, y: 558 } });
    await page.mouse.move(0, 0);
    await settle();
    const hinted = await canvas.screenshot();
    assert.equal(hinted.equals(initial), false, 'hint must fill the current row');
    await page.locator('#keyboard button[data-key="Enter"]').click();
    await settle();
    assert.equal((await canvas.screenshot()).equals(hinted), false, 'submitting a hint must grade it');
    await canvas.click({ position: { x: 270, y: 558 } });
    await page.mouse.move(0, 0);
    await settle();
    assert.equal((await canvas.screenshot()).equals(initial), true, 'reset must restore the board');
    assert.deepEqual(errors, []);
    console.log('Browser checks passed: WASM validation, startup, typing, touch keyboard, hint, submission, reset.');
  } finally {
    if (browser) await browser.close();
    await new Promise(resolve => server.close(resolve));
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
