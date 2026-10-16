/** Appearance, safety, the package list, adb and what adbDebloat is. */
import { useEffect, useState } from 'preact/hooks';
import { api } from '../api';
import type { Settings as SettingsType } from '../api/types';
import { formatNumber, LANGUAGES, t } from '../i18n';
import { errorMessage } from '../lib/errors';
import { formatAgo } from '../lib/format';
import { confirm } from '../state/dialogs';
import { loadPackages, refreshStatus, status } from '../state/device';
import { saveSettings, settings } from '../state/settings';
import { toast } from '../state/toasts';
import { Button } from '../components/Button';
import { PageHeader, Segmented, TextField } from '../components/Misc';
import { Switch } from '../components/Switch';

export const REPO_URL = 'https://github.com/bugragungoz/adbDebloat';
const COMMUNITY_PAGE = 'https://github.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation';

function Group({ title, children }: { title: string; children: preact.ComponentChildren }) {
  return (
    <section class="settings-group">
      <h2 class="settings-title">{title}</h2>
      <div class="settings-body">{children}</div>
    </section>
  );
}

export function Settings() {
  const s = settings.value;
  const st = status.value;
  const [adbPath, setAdbPath] = useState(s?.adbPath ?? '');
  const [busy, setBusy] = useState(false);
  useEffect(() => setAdbPath(s?.adbPath ?? ''), [s?.adbPath]);
  if (!s) return null;

  const save = async (patch: Partial<SettingsType>, reload = false) => {
    try {
      await saveSettings(patch);
      if (reload) void loadPackages();
    } catch (e) {
      toast(errorMessage(e), { tone: 'warn' });
    }
  };

  const setExpert = async (on: boolean) => {
    if (on) {
      const ok = await confirm({
        title: t('settings.expertTitle'),
        body: t('settings.expertBody'),
        confirmLabel: t('settings.expertConfirm'),
        cancelLabel: t('common.cancel'),
        danger: true,
      });
      if (!ok) return;
    }
    await save({ expertMode: on }, true);
  };

  const refreshList = async () => {
    setBusy(true);
    try {
      const k = await api.refreshKnowledge();
      toast(t('settings.listUpdated', { count: formatNumber(k.packages) }), { tone: 'ok' });
      await refreshStatus();
      void loadPackages();
    } catch (e) {
      toast(errorMessage(e), { tone: 'warn' });
    } finally {
      setBusy(false);
    }
  };

  const saveAdbPath = async () => {
    await save({ adbPath });
    await refreshStatus();
  };

  const k = st?.knowledge;
  const adb = st?.adb;
  return (
    <div class="view-body">
      <PageHeader title={t('settings.title')} />

      <Group title={t('settings.appearance')}>
        <div class="setting-row">
          <span class="setting-label">{t('settings.theme')}</span>
          <Segmented
            label={t('settings.theme')}
            value={s.theme}
            onChange={(v) => void save({ theme: v })}
            options={[
              { value: 'system', label: t('settings.themeSystem') },
              { value: 'dark', label: t('settings.themeDark') },
              { value: 'light', label: t('settings.themeLight') },
            ]}
          />
        </div>
        <div class="setting-row">
          <span class="setting-label">{t('settings.language')}</span>
          <Segmented
            label={t('settings.language')}
            value={s.language}
            onChange={(v) => void save({ language: v })}
            options={[{ value: 'system' as const, label: t('settings.languageSystem') }, ...LANGUAGES.map((l) => ({ value: l.code as SettingsType['language'], label: l.nativeName }))]}
          />
        </div>
      </Group>

      <Group title={t('settings.safety')}>
        <Switch checked={s.expertMode} onChange={(v) => void setExpert(v)} label={t('settings.expert')} hint={t('settings.expertHint')} />
      </Group>

      <Group title={t('settings.list')}>
        <Switch checked={s.communityList} onChange={(v) => void save({ communityList: v }, true)} label={t('settings.community')} hint={t('settings.communityHint')} />
        {k ? (
          <p class="setting-meta">
            {k.source === 'community'
              ? t('settings.listCommunity', { count: formatNumber(k.packages), ago: formatAgo(k.fetchedAt, Date.now()) })
              : t('settings.listBuiltin', { count: formatNumber(k.packages) })}
          </p>
        ) : null}
        <div class="actions">
          {s.communityList ? (
            <Button icon="refresh" onClick={() => void refreshList()} disabled={busy}>
              {t('settings.updateList')}
            </Button>
          ) : null}
          <Button variant="quiet" icon="external" onClick={() => void api.openUrl(COMMUNITY_PAGE)}>
            {t('settings.communityPage')}
          </Button>
        </div>
      </Group>

      <Group title={t('settings.adb')}>
        <p class="setting-meta">
          {adb ? t('settings.adbInUse', { version: adb.version, source: t(`settings.adbSource.${adb.source}`) }) : t('settings.adbNone')}
          {adb ? (
            <>
              <br />
              <span class="mono">{adb.path}</span>
            </>
          ) : null}
        </p>
        <div class="form-row">
          <TextField id="adb-path" label={t('settings.adbPath')} value={adbPath} onInput={setAdbPath} placeholder={t('settings.adbPathPlaceholder')} mono />
          <Button onClick={() => void saveAdbPath()} disabled={adbPath.trim() === s.adbPath}>
            {t('settings.save')}
          </Button>
        </div>
        <p class="setting-meta">{t('settings.adbPathHint')}</p>
      </Group>

      <Group title={t('settings.about')}>
        <p class="about-text">{t('settings.aboutBody')}</p>
        <p class="setting-meta">{t('settings.version', { version: __APP_VERSION__ })}</p>
        <div class="actions">
          <Button variant="quiet" icon="github" onClick={() => void api.openUrl(REPO_URL)}>
            {t('settings.source')}
          </Button>
        </div>
        <p class="setting-meta">{t('settings.license')}</p>
      </Group>
    </div>
  );
}
