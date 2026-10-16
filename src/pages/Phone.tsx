/**
 * The phone: getting adb, connecting a phone by cable or Wi-Fi, and what the connected phone is.
 */
import { useEffect, useState } from 'preact/hooks';
import { api } from '../api';
import type { DeviceEntry } from '../api/types';
import { formatNumber, t } from '../i18n';
import { errorMessage } from '../lib/errors';
import { formatBytes } from '../lib/format';
import { confirm } from '../state/dialogs';
import { choose, current, device, devices, packages, refreshStatus, status } from '../state/device';
import { toast } from '../state/toasts';
import { page } from '../state/ui';
import { Button } from '../components/Button';
import { Icon } from '../components/Icon';
import { Meter, Note, PageHeader, TextField } from '../components/Misc';

const TERMS_URL = 'https://developer.android.com/studio/releases/platform-tools';

function GetAdb() {
  const [progress, setProgress] = useState<{ done: number; total: number | null } | null>(null);
  const [busy, setBusy] = useState(false);
  const download = async () => {
    setBusy(true);
    setProgress({ done: 0, total: null });
    const off = await api.onDownloadProgress(setProgress);
    try {
      const tool = await api.downloadPlatformTools();
      toast(t('phone.adbReady', { version: tool.version }), { tone: 'ok' });
      await refreshStatus();
    } catch (e) {
      toast(errorMessage(e), { tone: 'warn' });
    } finally {
      off();
      setBusy(false);
      setProgress(null);
    }
  };
  const broken = status.value?.adbProblem?.code === 'broken';
  return (
    <section class="card">
      <h2 class="card-title">{t(broken ? 'phone.adbBrokenTitle' : 'phone.adbMissingTitle')}</h2>
      <p class="card-text">{broken ? t('phone.adbBrokenBody', { detail: status.value?.adbProblem?.detail ?? '' }) : t('phone.adbMissingBody')}</p>
      {progress ? (
        <div class="progress-line">
          <Meter value={progress.total ? progress.done / progress.total : 0} label={t('phone.downloading')} />
          <span>
            {progress.total
              ? t('phone.downloadedOf', { done: formatBytes(progress.done), total: formatBytes(progress.total) })
              : t('phone.downloading')}
          </span>
        </div>
      ) : null}
      <div class="actions">
        <Button variant="primary" icon="download" onClick={() => void download()} disabled={busy}>
          {t('phone.download')}
        </Button>
        <Button variant="quiet" icon="settings" onClick={() => (page.value = 'settings')}>
          {t('phone.haveAdb')}
        </Button>
      </div>
      <p class="setting-meta">
        {t('phone.terms')}{' '}
        <button type="button" class="link-button" onClick={() => void api.openUrl(TERMS_URL)}>
          {t('phone.termsLink')}
        </button>
      </p>
    </section>
  );
}

function ConnectSteps() {
  return (
    <section class="card">
      <h2 class="card-title">{t('phone.connectTitle')}</h2>
      <ol class="steps">
        <li>{t('phone.step1')}</li>
        <li>{t('phone.step2')}</li>
        <li>{t('phone.step3')}</li>
        <li>{t('phone.step4')}</li>
      </ol>
      <p class="setting-meta">{t('phone.xiaomiHint')}</p>
    </section>
  );
}

function Wireless() {
  const [pairAddress, setPairAddress] = useState('');
  const [code, setCode] = useState('');
  const [address, setAddress] = useState('');
  const [busy, setBusy] = useState(false);
  const run = async (work: () => Promise<string>, ok: string) => {
    setBusy(true);
    try {
      await work();
      toast(ok, { tone: 'ok' });
      await refreshStatus();
    } catch (e) {
      toast(errorMessage(e), { tone: 'warn' });
    } finally {
      setBusy(false);
    }
  };
  return (
    <section class="card">
      <h2 class="card-title">{t('phone.wifiTitle')}</h2>
      <p class="card-text">{t('phone.wifiBody')}</p>
      <div class="form-row">
        <TextField id="pair-address" label={t('phone.pairAddress')} value={pairAddress} onInput={setPairAddress} placeholder="192.168.1.23:37145" mono />
        <TextField id="pair-code" label={t('phone.pairCode')} value={code} onInput={setCode} placeholder="123456" inputMode="numeric" mono />
        <Button onClick={() => void run(() => api.pair(pairAddress, code), t('phone.paired'))} disabled={busy || !pairAddress.trim() || !code.trim()}>
          {t('phone.pair')}
        </Button>
      </div>
      <div class="form-row">
        <TextField id="connect-address" label={t('phone.connectAddress')} value={address} onInput={setAddress} placeholder="192.168.1.23:41235" mono />
        <Button onClick={() => void run(() => api.connect(address), t('phone.connected'))} disabled={busy || !address.trim()}>
          {t('phone.connect')}
        </Button>
      </div>
    </section>
  );
}

function stateNote(d: DeviceEntry) {
  if (d.state === 'unauthorized') {
    return (
      <Note icon="phone" tone="warn" title={t('phone.unauthorizedTitle')}>
        {t('phone.unauthorizedBody')}
      </Note>
    );
  }
  if (d.state === 'offline') {
    return (
      <Note icon="alert" tone="warn" title={t('phone.offlineTitle')}>
        {t('phone.offlineBody')}
      </Note>
    );
  }
  if (d.state === 'noPermission') {
    return (
      <Note icon="alert" tone="warn" title={t('phone.noPermissionTitle')}>
        {t('phone.noPermissionBody')}
      </Note>
    );
  }
  if (d.state === 'otherMode') {
    return (
      <Note icon="alert" tone="warn" title={t('phone.otherModeTitle')}>
        {t('phone.otherModeBody')}
      </Note>
    );
  }
  return null;
}

function PhoneCard({ entry }: { entry: DeviceEntry }) {
  const info = device.value?.serial === entry.serial ? device.value : null;
  const [busy, setBusy] = useState(false);
  const pkgs = packages.value;
  const counts = pkgs
    ? {
        total: pkgs.length,
        removed: pkgs.filter((p) => p.state === 'removed').length,
        disabled: pkgs.filter((p) => p.state === 'disabled').length,
      }
    : null;

  const reboot = async () => {
    const ok = await confirm({
      title: t('phone.rebootTitle'),
      body: t('phone.rebootBody'),
      confirmLabel: t('phone.reboot'),
      cancelLabel: t('common.cancel'),
    });
    if (!ok) return;
    try {
      await api.reboot(entry.serial);
      toast(t('phone.rebooting'), { tone: 'ok' });
    } catch (e) {
      toast(errorMessage(e), { tone: 'warn' });
    }
  };
  const toWifi = async () => {
    setBusy(true);
    try {
      const address = await api.switchToWifi(entry.serial);
      toast(t('phone.onWifi', { address }), { tone: 'ok' });
      await refreshStatus();
    } catch (e) {
      toast(errorMessage(e), { tone: 'warn' });
    } finally {
      setBusy(false);
    }
  };
  const disconnect = async () => {
    try {
      await api.disconnect(entry.serial);
      await refreshStatus();
    } catch (e) {
      toast(errorMessage(e), { tone: 'warn' });
    }
  };

  const name = info?.name || (entry.model ? entry.model.replace(/_/g, ' ') : entry.serial);
  return (
    <section class="card phone-card" aria-label={name}>
      <span class="phone-art" aria-hidden="true">
        <Icon name="phone" size={34} />
      </span>
      <div class="phone-main">
        <div>
          <h2 class="phone-name">{name}</h2>
          <p class="phone-sub">
            {entry.state === 'ready'
              ? t(entry.transport === 'wifi' ? 'phone.readyWifi' : 'phone.readyUsb')
              : t(`phone.state.${entry.state}`)}
          </p>
        </div>
        {stateNote(entry)}
        {info ? (
          <dl class="facts">
            <div>
              <dt>{t('phone.android')}</dt>
              <dd>{t('phone.androidValue', { version: info.android, sdk: info.sdk })}</dd>
            </div>
            {info.skin ? (
              <div>
                <dt>{t('phone.skin')}</dt>
                <dd>{info.skin}</dd>
              </div>
            ) : null}
            <div>
              <dt>{t('phone.brand')}</dt>
              <dd>{info.manufacturer || info.brand}</dd>
            </div>
            <div>
              <dt>{t('phone.codename')}</dt>
              <dd class="mono">{info.codename}</dd>
            </div>
            <div>
              <dt>{t('phone.serial')}</dt>
              <dd class="mono">{info.serial}</dd>
            </div>
            {info.battery !== null ? (
              <div>
                <dt>{t('phone.battery')}</dt>
                <dd>{t('phone.batteryValue', { level: info.battery })}</dd>
              </div>
            ) : null}
            {counts ? (
              <div>
                <dt>{t('phone.packages')}</dt>
                <dd>{t('phone.packagesValue', { total: formatNumber(counts.total), off: formatNumber(counts.removed + counts.disabled) })}</dd>
              </div>
            ) : null}
          </dl>
        ) : null}
        {entry.state === 'ready' ? (
          <div class="actions">
            <Button variant="primary" icon="apps" onClick={() => (page.value = 'apps')}>
              {t('phone.toApps')}
            </Button>
            {entry.transport === 'usb' ? (
              <Button icon="wifi" onClick={() => void toWifi()} disabled={busy}>
                {t('phone.useWifi')}
              </Button>
            ) : null}
            {entry.transport === 'wifi' ? (
              <Button icon="unlink" onClick={() => void disconnect()}>
                {t('phone.disconnect')}
              </Button>
            ) : null}
            <Button variant="quiet" icon="power" onClick={() => void reboot()}>
              {t('phone.reboot')}
            </Button>
          </div>
        ) : null}
      </div>
    </section>
  );
}

export function Phone() {
  useEffect(() => void refreshStatus(), []);
  const s = status.value;
  const list = devices.value;
  const d = current.value;

  return (
    <div class="view-body">
      <PageHeader title={t('phone.title')} subtitle={t('phone.subtitle')} />
      {!s ? null : !s.adb ? (
        <GetAdb />
      ) : (
        <>
          {d ? <PhoneCard entry={d} /> : <ConnectSteps />}
          {list.length > 1 ? (
            <section class="card">
              <h2 class="card-title">{t('phone.otherPhones')}</h2>
              <ul class="device-list">
                {list.map((x) => (
                  <li key={x.serial}>
                    <button type="button" class="device-row" aria-current={x.serial === d?.serial} onClick={() => choose(x.serial)}>
                      <Icon name={x.transport === 'wifi' ? 'wifi' : 'usb'} size={16} />
                      <span class="grow">{x.model ? x.model.replace(/_/g, ' ') : x.serial}</span>
                      <span class="muted mono">{x.serial}</span>
                      <span class="muted">{t(`phone.state.${x.state}`)}</span>
                    </button>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
          <Wireless />
        </>
      )}
    </div>
  );
}
