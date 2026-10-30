import { expect, test } from '@playwright/test';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const host = `http://127.0.0.1:${process.env.OPENRIDR_E2E_HOST_PORT ?? '4317'}`;
test('OrderDesk editor comparison', async ({ page, request }) => {
  test.setTimeout(90000);
  const base = fs.mkdtempSync(path.join(os.tmpdir(), 'openridr-rider-visual-'));
  const copy = path.join(base, 'OrderDesk');
  fs.cpSync(path.resolve('fixtures/OrderDesk'), copy, { recursive: true, filter: (p) => !/[\\/](bin|obj)([\\/]|$)/.test(p) });
  fs.appendFileSync(path.join(copy, 'src/OrderDesk.Cli/Program.cs'), '\n// edit\n');
  try {
    await request.post(`${host}/call`, { data: { method: 'workspace.close', params: {} } });
    await page.setViewportSize({ width: 1710, height: 981 });
    await page.goto('/');
    await page.getByTestId('workspace-path-input').fill(path.join(copy, 'OrderDesk.sln'));
    await page.getByTestId('workspace-open').click();
    await page.getByTestId('trust-checkbox').check();
    await page.getByTestId('trust-grant').click();
    await page.getByTestId('open-text-only').click();
    await expect(page.getByTestId('eval-state')).toContainText('Evaluated', { timeout: 30000 });
    await page.getByTestId('tree-file-src/OrderDesk.Cli/Program.cs').click();
    await expect(page.getByTestId('editor')).toContainText('// edit');
    await page.getByTestId('bottom-hide').click();
    await expect(page.getByRole('region', { name: 'Bottom panel' })).toHaveCount(0);
    await page.screenshot({ path: '/tmp/openridr-rider-editor-comparison.png' });
  } finally {
    await request.post(`${host}/call`, { data: { method: 'workspace.close', params: {} } });
    fs.rmSync(base, { recursive: true, force: true });
  }
});
