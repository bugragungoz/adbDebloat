/** Turns an error from the Rust side into a sentence for the owner. */
import { toAppError } from '../api/invoke';
import { t } from '../i18n';

const KNOWN = new Set(['no_adb', 'adb_failed', 'busy', 'invalid', 'download_failed', 'list_failed', 'history_failed', 'settings_not_saved']);

export function errorMessage(e: unknown): string {
  const err = toAppError(e);
  const code = KNOWN.has(err.code) ? err.code : 'internal';
  return t(`errors.${code}`, { detail: err.detail || err.message });
}
