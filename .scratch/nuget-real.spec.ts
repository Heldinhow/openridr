import { expect, test } from '@playwright/test';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const host = `http://127.0.0.1:${process.env.OPENRIDR_E2E_HOST_PORT ?? '4317'}`;
test('real NuGet listing', async ({ page, request }) => {
  test.setTimeout(180_000);
  const base = fs.mkdtempSync(path.join(os.tmpdir(), 'openridr-nuget-real-'));
  const copy = path.join(base, 'OrderDesk');
  fs.cpSync(path.resolve('fixtures/OrderDesk'), copy, { recursive: true, filter: (p) => !/[\\/](bin|obj)([\\/]|$)/.test(p) });
  try {
    await request.post(`${host}/call`, { data: { method: 'workspace.close', params: {} } });
    await page.goto('/');
    await page.getByTestId('workspace-path-input').fill(path.join(copy, 'OrderDesk.sln'));
    await page.getByTestId('workspace-open').click();
    await page.getByTestId('trust-checkbox').check();
    await page.getByTestId('trust-grant').click();
    await page.getByTestId('open-text-only').click();
    await page.getByTestId('workspace-tools-toggle').click();
    await expect(page.getByTestId('eval-state')).toContainText('Evaluated', { timeout: 30000 });
    await page.getByTestId('restore-action').click();
    await expect(page.getByTestId('restore-state')).toContainText('Restore succeeded', { timeout: 90000 });
    await page.getByTestId('restore-reload').click();
    await expect(page.getByTestId('eval-state')).toContainText('revision 2', { timeout: 30000 });
    await page.getByTestId('bottom-tab-nuget').click();
    await expect(page.getByTestId('nuget-probe')).toHaveCount(0);
    await expect(page.getByTestId('nuget-refresh')).toBeEnabled({ timeout: 90000 });
    await page.getByTestId('nuget-project').selectOption({ label: 'OrderDesk.Tests' });
    await expect(page.getByTestId('nuget-refresh')).toBeEnabled({ timeout: 90000 });
    await expect(page.getByTestId('nuget-panel')).toContainText('xunit', { timeout: 90000 });
    await expect(page.getByTestId('nuget-sources')).toContainText('https://api.nuget.org/v3/index.json');
    await expect(page.getByTestId('nuget-sources')).toContainText('Package origin is not reported');
    await expect(page.getByTestId('nuget-sources')).not.toContainText('source query failed');
    await page.screenshot({ path: '/tmp/openridr-rider-nuget-real.png' });
    console.log('Summary:', await page.getByTestId('nuget-summary').innerText());
  } finally {
    await request.post(`${host}/call`, { data: { method: 'workspace.close', params: {} } });
    fs.rmSync(base, { recursive: true, force: true });
  }
});
