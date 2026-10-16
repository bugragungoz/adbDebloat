/** Sizes and times for people. Sizes are decimal (1 MB = 1,000,000 bytes). */
import { formatNumber, locale, t } from '../i18n';

const BYTE_UNITS = ['B', 'KB', 'MB', 'GB'];

/** One decimal below 10, none above. */
export function formatBytes(bytes: number): string {
  let i = 0;
  let v = Math.max(0, bytes);
  while (v >= 1000 && i < BYTE_UNITS.length - 1) {
    v /= 1000;
    i++;
  }
  const digits = i === 0 || v >= 10 ? 0 : 1;
  return `${formatNumber(v, { maximumFractionDigits: digits, minimumFractionDigits: digits })} ${BYTE_UNITS[i]}`;
}

/** "just now", "4 min ago", "2 h ago", "3 days ago". */
export function formatAgo(at: number | null | undefined, now: number): string {
  if (!at) return t('time.never');
  const s = Math.max(0, Math.round((now - at) / 1000));
  if (s < 60) return t('time.now');
  if (s < 3600) return t('time.minutesAgo', { count: Math.floor(s / 60) });
  if (s < 86_400) return t('time.hoursAgo', { count: Math.floor(s / 3600) });
  return t('time.daysAgo', { count: Math.floor(s / 86_400) });
}

/** A date and time in the owner's language: "16 Oct 2026, 15:42". */
export function formatDateTime(at: number): string {
  return new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium', timeStyle: 'short' }).format(at);
}
