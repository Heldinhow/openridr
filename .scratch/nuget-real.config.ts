import { defineConfig } from '@playwright/test';
import base from '../playwright.config';
export default defineConfig({ ...base, testDir: '.', testMatch: 'nuget-real.spec.ts' });
