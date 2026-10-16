/** Every batch run so far, with what happened to each package and a way to undo it. */
import { useEffect, useState } from 'preact/hooks';
import type { Session } from '../api/types';
import { formatNumber, t } from '../i18n';
import { formatDateTime } from '../lib/format';
import { openReview } from '../state/batch';
import { device, ready } from '../state/device';
import { loadHistory, sessions } from '../state/history';
import { Button } from '../components/Button';
import { Icon } from '../components/Icon';
import { EmptyState, PageHeader } from '../components/Misc';
import { failureText } from './Review';

function Row({ session }: { session: Session }) {
  const [open, setOpen] = useState(false);
  const ok = session.items.filter((i) => i.ok).length;
  const failed = session.items.length - ok;
  const undoable = session.action !== 'restore' && ok > 0;
  const here = ready.value && (ready.value.serial === session.device.serial || device.value?.codename === session.device.codename);
  return (
    <li class="history-row">
      <div class="history-head">
        <Icon name={session.action === 'restore' ? 'restore' : session.action === 'remove' ? 'trash' : 'ban'} size={18} />
        <span class="history-title">{t(`history.batch.${session.action}`, { count: session.items.length })}</span>
        <span class="history-meta">
          {session.device.name} - {formatDateTime(session.at)}
        </span>
        <span class="grow" />
        <span class="badge badge-ok">{t('history.ok', { count: formatNumber(ok) })}</span>
        {failed ? <span class="badge badge-warn">{t('history.failed', { count: formatNumber(failed) })}</span> : null}
      </div>
      <div class="actions">
        <Button size="sm" variant="quiet" icon={open ? 'chevronDown' : 'chevronRight'} onClick={() => setOpen(!open)} aria-expanded={open}>
          {open ? t('history.hide') : t('history.show')}
        </Button>
        {undoable ? (
          <Button
            size="sm"
            icon="restore"
            disabled={!here}
            title={here ? undefined : t('history.otherPhone', { name: session.device.name })}
            onClick={() => openReview('restore', session.items.filter((i) => i.ok).map((i) => i.package), session)}
          >
            {t('history.undo')}
          </Button>
        ) : null}
      </div>
      {open ? (
        <ul class="history-items">
          {session.items.map((i) => (
            <li key={i.package}>
              <span class={`result-icon ${i.ok ? 'is-ok' : 'is-failed'}`}>
                <Icon name={i.ok ? 'checkCircle' : 'alert'} size={14} title={i.ok ? t('review.ok') : t('review.failed')} />
              </span>
              <span class="pkg-text">
                <span class="truncate">{i.label ?? i.package}</span>
                {i.label ? <span class="pkg-name mono muted truncate">{i.package}</span> : null}
                {i.ok ? null : <span class="muted">{failureText(i)}</span>}
              </span>
              <span />
            </li>
          ))}
        </ul>
      ) : null}
    </li>
  );
}

export function History() {
  useEffect(() => void loadHistory(), []);
  const list = sessions.value;
  return (
    <div class="view-body">
      <PageHeader title={t('history.title')} subtitle={t('history.subtitle')} />
      {list.length === 0 ? (
        <EmptyState icon="history" title={t('history.empty')} body={t('history.emptyBody')} />
      ) : (
        <ul class="history-list">
          {list.map((s) => (
            <Row key={s.id} session={s} />
          ))}
        </ul>
      )}
    </div>
  );
}
