/**
 * The review dialog: what will change and what will be skipped (and why) before anything is sent
 * to the phone, then each package's result as it comes in.
 */
import type { Action, AppRow, ItemResult, Removal } from '../api/types';
import { collator, formatNumber, t } from '../i18n';
import { closeReview, review, run, startBatch, stopBatch } from '../state/batch';
import { packages } from '../state/device';
import { settings } from '../state/settings';
import { Button } from '../components/Button';
import { Dialog } from '../components/Dialog';
import { Icon } from '../components/Icon';
import { Note } from '../components/Misc';
import { displayName, lockReason, SafetyBadge } from '../components/Safety';
import { canApply } from './Apps';

const RISK: Record<Removal | 'unknown', number> = { unsafe: 0, expert: 1, unknown: 2, advanced: 3, recommended: 4 };

/** Why a package is left out of a batch. */
function skipReason(action: Action, r: AppRow): string {
  const lock = action === 'restore' ? null : lockReason(r);
  if (lock) return lock;
  if (action === 'remove') return t('review.alreadyRemoved');
  if (action === 'disable') return t(r.state === 'removed' ? 'review.removedNotDisabled' : 'review.alreadyDisabled');
  return t('review.alreadyInstalled');
}

export function failureText(item: ItemResult): string {
  if (!item.failure) return '';
  const base = t(`failure.${item.failure.code}`);
  return item.failure.detail && item.failure.code !== 'protected' && item.failure.code !== 'needs_expert' ? `${base} ${item.failure.detail}` : base;
}

function Results() {
  const r = run.value!;
  const rows = packages.value ?? [];
  const label = (name: string, fallback?: string) => {
    const row = rows.find((x) => x.name === name);
    return row ? displayName(row) : fallback || name;
  };
  const ok = r.results.filter((i) => i.ok).length;
  const failed = r.results.length - ok;
  const done = r.session !== null || r.error !== null;
  return (
    <div class="review-groups">
      <p class="review-intro" role="status">
        {done
          ? t('review.finished', { ok: formatNumber(ok), failed: formatNumber(failed) })
          : t('review.progress', { done: formatNumber(r.results.length), total: formatNumber(r.total) })}
      </p>
      {r.error ? (
        <Note icon="alert" tone="warn" title={t('review.stopped')}>
          {r.error}
        </Note>
      ) : null}
      <ul class="review-list" aria-label={t('review.results')}>
        {r.results.map((i) => (
          <li key={i.package} class="review-item">
            <span class="pkg-text">
              <span class="truncate">{label(i.package, i.label)}</span>
              <span class="pkg-name mono muted truncate">{i.package}</span>
            </span>
            <span class={`result-icon ${i.ok ? 'is-ok' : 'is-failed'}`}>
              <Icon name={i.ok ? 'checkCircle' : 'alert'} size={16} title={i.ok ? t('review.ok') : t('review.failed')} />
            </span>
            {i.ok ? null : <span class="review-reason">{failureText(i)}</span>}
          </li>
        ))}
      </ul>
      {failed && r.action === 'disable' && r.results.some((i) => i.failure?.code === 'disable_refused') ? <p class="setting-meta">{t('review.tryRemove')}</p> : null}
    </div>
  );
}

export function ReviewDialog() {
  const req = review.value;
  if (!req) return null;
  const r = run.value;
  const all = packages.value ?? [];
  const rows = req.packages
    .map((name) => all.find((x) => x.name === name))
    .filter((x): x is AppRow => !!x)
    // The ones to read twice come first.
    .sort((a, b) => RISK[a.removal ?? 'unknown'] - RISK[b.removal ?? 'unknown'] || collator.value.compare(displayName(a), displayName(b)));
  const missing = req.packages.filter((name) => !all.some((x) => x.name === name));
  const go = rows.filter((x) => canApply(req.action, x));
  const skip = rows.filter((x) => !canApply(req.action, x));
  const risky = go.filter((x) => x.removal === 'advanced' || x.removal === 'expert' || x.removal === null);
  const running = r !== null && r.session === null && r.error === null;
  const finished = r !== null && !running;
  const title = finished
    ? t('review.doneTitle')
    : running
      ? t(`review.running.${req.action}`, { count: r.total })
      : req.undo
        ? t('review.undoTitle')
        : t(`review.title.${req.action}`, { count: go.length });

  return (
    <Dialog
      title={title}
      size="list"
      onClose={closeReview}
      busy={running}
      actions={
        finished ? (
          <Button variant="primary" onClick={closeReview}>
            {t('common.close')}
          </Button>
        ) : running ? (
          <Button onClick={stopBatch}>{t('review.stop')}</Button>
        ) : (
          <>
            <Button data-cancel onClick={closeReview}>
              {t('common.cancel')}
            </Button>
            <Button
              variant={req.action === 'restore' ? 'primary' : 'danger'}
              disabled={!go.length}
              onClick={() => void startBatch(go.map((x) => x.name))}
            >
              {t(`review.start.${req.action}`, { count: go.length })}
            </Button>
          </>
        )
      }
    >
      {r ? (
        <Results />
      ) : (
        <div class="review-groups">
          <p class="review-intro">{t(`review.explain.${req.action}`)}</p>
          {risky.length && req.action !== 'restore' ? (
            <Note icon="alert" tone="warn" title={t('review.riskyTitle', { count: risky.length })}>
              {t('review.riskyBody')}
            </Note>
          ) : null}
          {settings.value?.expertMode && go.some((x) => x.removal === 'expert') && req.action !== 'restore' ? (
            <Note icon="alert" tone="warn" title={t('review.expertTitle')}>
              {t('review.expertBody')}
            </Note>
          ) : null}
          {go.length ? (
            <section class="review-group">
              <h3>
                <Icon name={req.action === 'restore' ? 'restore' : req.action === 'remove' ? 'trash' : 'ban'} size={14} />
                {t(`review.will.${req.action}`, { count: go.length })}
              </h3>
              <ul class="review-list">
                {go.map((x) => (
                  <li key={x.name} class="review-item">
                    <span class="pkg-text">
                      <span class="truncate">{displayName(x)}</span>
                      <span class="pkg-name mono muted truncate">{x.name}</span>
                    </span>
                    <SafetyBadge removal={x.removal} />
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
          {skip.length || missing.length ? (
            <section class="review-group">
              <h3>
                <Icon name="lock" size={14} />
                {t('review.skipped', { count: skip.length + missing.length })}
              </h3>
              <ul class="review-list">
                {skip.map((x) => (
                  <li key={x.name} class="review-item">
                    <span class="pkg-text">
                      <span class="truncate">{displayName(x)}</span>
                      <span class="pkg-name mono muted truncate">{x.name}</span>
                    </span>
                    <SafetyBadge removal={x.removal} />
                    <span class="review-reason">{skipReason(req.action, x)}</span>
                  </li>
                ))}
                {missing.map((name) => (
                  <li key={name} class="review-item">
                    <span class="pkg-name mono truncate">{name}</span>
                    <span />
                    <span class="review-reason">{t('failure.unknown')}</span>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
        </div>
      )}
    </Dialog>
  );
}
