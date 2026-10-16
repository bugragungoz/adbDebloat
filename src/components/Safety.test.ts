import { describe, expect, it } from 'vitest';
import type { AppRow } from '../api/types';
import { setLanguage } from '../i18n';
import { canApply } from '../pages/Apps';
import { displayName, firstLine, listLabel, lockReason } from './Safety';

function row(over: Partial<AppRow> = {}): AppRow {
  return {
    name: 'com.miui.msa.global',
    label: 'MIUI System Ads',
    path: '/system/app/MSA/MSA.apk',
    system: true,
    state: 'installed',
    removal: 'recommended',
    list: 'oem',
    description: 'Shows ads. And more.\nSecond line.',
    dependencies: [],
    neededBy: [],
    verdict: 'allowed',
    protection: null,
    ...over,
  };
}

describe('what can be done to a package', () => {
  it('follows its state', () => {
    expect(canApply('remove', row())).toBe(true);
    expect(canApply('disable', row())).toBe(true);
    expect(canApply('restore', row())).toBe(false);
    expect(canApply('remove', row({ state: 'disabled' }))).toBe(true);
    expect(canApply('disable', row({ state: 'disabled' }))).toBe(false);
    expect(canApply('restore', row({ state: 'disabled' }))).toBe(true);
    expect(canApply('remove', row({ state: 'removed' }))).toBe(false);
    expect(canApply('restore', row({ state: 'removed' }))).toBe(true);
  });

  it('never removes or disables a protected package, but always restores it', () => {
    const kept = row({ verdict: 'protected', protection: 'launcher', state: 'installed' });
    expect(canApply('remove', kept)).toBe(false);
    expect(canApply('disable', kept)).toBe(false);
    expect(canApply('restore', row({ verdict: 'protected', protection: 'core', state: 'removed' }))).toBe(true);
    expect(canApply('remove', row({ verdict: 'needsExpert' }))).toBe(false);
  });

  it('says why a package is kept', () => {
    setLanguage('en');
    expect(lockReason(row())).toBeNull();
    expect(lockReason(row({ verdict: 'protected', protection: 'keyboard' }))).toMatch(/keyboard in use/);
    expect(lockReason(row({ verdict: 'needsExpert' }))).toMatch(/expert mode/);
  });
});

describe('names and descriptions', () => {
  it('prefers the label and shortens a description to its first sentence', () => {
    expect(displayName(row())).toBe('MIUI System Ads');
    expect(displayName(row({ label: null }))).toBe('com.miui.msa.global');
    expect(firstLine(row().description)).toBe('Shows ads.');
    expect(firstLine('One line without a stop')).toBe('One line without a stop');
    expect(firstLine(null)).toBe('');
  });

  it('names who ships a package, and keeps a list it does not know as it is', () => {
    setLanguage('en');
    expect(listLabel('oem')).toBe('Phone maker');
    expect(listLabel('pending')).toBe('pending');
  });
});
