import { afterEach, describe, expect, it } from 'vitest';
import { formatNumber, setLanguage, t } from '.';

afterEach(() => setLanguage('en'));

describe('t', () => {
  it('fills placeholders and picks the plural form from count', () => {
    setLanguage('en');
    expect(t('apps.count', { count: 1 })).toBe('1 package');
    expect(t('apps.count', { count: 12 })).toBe('12 packages');
    expect(t('review.title.remove', { count: 3 })).toBe('Remove 3 apps?');
    expect(t('review.title.remove', { count: 1 })).toBe('Remove 1 app?');
  });

  it('writes numbers the way the language does', () => {
    setLanguage('tr');
    expect(formatNumber(1234.5)).toBe('1.234,5');
    expect(t('apps.count', { count: 1200 })).toBe('1.200 paket');
  });

  it('switches language and falls back to the key for an unknown message', () => {
    setLanguage('tr');
    expect(t('nav.apps')).toBe('Uygulamalar');
    expect(t('no.such.key')).toBe('no.such.key');
    setLanguage('xx');
    expect(t('nav.apps')).toBe('Apps');
  });

  it('leaves a placeholder without a value as it is', () => {
    setLanguage('en');
    expect(t('history.otherPhone')).toBe('Connect {name} to undo this batch.');
  });
});
