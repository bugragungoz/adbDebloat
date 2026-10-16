/**
 * The packages on the phone: filters, a table sorted with the safest to remove first, a selection
 * that goes to the review dialog, and a panel with everything known about one package.
 */
import { useComputed } from '@preact/signals';
import type { Action, AppRow, Removal } from '../api/types';
import { collator, formatNumber, t } from '../i18n';
import { openReview } from '../state/batch';
import { loadPackages, loadingPackages, packages, packagesError, ready } from '../state/device';
import {
  clearSelection,
  levels,
  openPackage,
  page,
  search,
  selection,
  setSelected,
  stateFilter,
  systemOnly,
  toggleLevel,
  type Level,
} from '../state/ui';
import { Button } from '../components/Button';
import { Chip } from '../components/Chip';
import { Icon } from '../components/Icon';
import { IconButton } from '../components/IconButton';
import { EmptyState, Note, PageHeader, SearchBox, Segmented } from '../components/Misc';
import { displayName, firstLine, listLabel, lockReason, SafetyBadge } from '../components/Safety';
import { Checkbox, Switch } from '../components/Switch';

const LEVEL_ORDER: Record<Level, number> = { recommended: 0, advanced: 1, expert: 2, unknown: 3, unsafe: 4 };
const LEVELS: Level[] = ['recommended', 'advanced', 'expert', 'unsafe', 'unknown'];
const levelOf = (r: AppRow): Level => r.removal ?? 'unknown';

/** Whether an action changes this package, and the rules let it. */
export function canApply(action: Action, r: AppRow): boolean {
  if (action === 'restore') return r.state !== 'installed';
  if (r.verdict !== 'allowed') return false;
  return action === 'remove' ? r.state !== 'removed' : r.state === 'installed';
}

function matches(r: AppRow, q: string): boolean {
  return r.name.toLowerCase().includes(q) || (r.label ?? '').toLowerCase().includes(q) || (r.description ?? '').toLowerCase().includes(q);
}

function StateText({ state }: { state: AppRow['state'] }) {
  return <span class={`state-text ${state === 'installed' ? 'is-installed' : ''}`}>{t(`state.${state}`)}</span>;
}

function Panel({ row, onClose }: { row: AppRow; onClose: () => void }) {
  const reason = lockReason(row);
  const all = packages.value ?? [];
  const label = (name: string) => {
    const r = all.find((x) => x.name === name);
    return r ? displayName(r) : name;
  };
  return (
    <aside class="detail-panel" aria-label={displayName(row)}>
      <header class="panel-head">
        <div class="panel-title-block">
          <h2 class="panel-title">{displayName(row)}</h2>
          {row.label ? <span class="panel-sub mono muted">{row.name}</span> : null}
        </div>
        <IconButton label={t('common.close')} icon="close" size="sm" onClick={onClose} />
      </header>
      <div class="panel-body">
        <div class="panel-badges">
          <SafetyBadge removal={row.removal} />
          <span class="badge">{t(`state.${row.state}`)}</span>
          <span class="badge">{t(row.system ? 'apps.system' : 'apps.user')}</span>
          {row.list ? <span class="badge">{listLabel(row.list)}</span> : null}
        </div>
        {reason ? (
          <Note icon="lock" tone="warn" title={t('apps.kept')}>
            {reason}
          </Note>
        ) : null}
        <section class="panel-section">
          <h3 class="panel-label">{t('apps.whatItIs')}</h3>
          <p class="panel-text">{row.description ?? t('apps.noDescription')}</p>
          {row.removal ? <p class="setting-meta">{t(`safety.${row.removal}Hint`)}</p> : null}
        </section>
        {row.neededBy.length ? (
          <section class="panel-section">
            <h3 class="panel-label">{t('apps.neededBy')}</h3>
            <ul class="pkg-links">
              {row.neededBy.map((n) => (
                <li key={n}>{label(n)}</li>
              ))}
            </ul>
          </section>
        ) : null}
        {row.dependencies.length ? (
          <section class="panel-section">
            <h3 class="panel-label">{t('apps.dependsOn')}</h3>
            <ul class="pkg-links">
              {row.dependencies.map((n) => (
                <li key={n}>{label(n)}</li>
              ))}
            </ul>
          </section>
        ) : null}
        {row.path ? (
          <section class="panel-section">
            <h3 class="panel-label">{t('apps.apk')}</h3>
            <p class="mono path">{row.path}</p>
          </section>
        ) : null}
        <div class="actions">
          {canApply('remove', row) ? (
            <Button variant="danger" icon="trash" onClick={() => openReview('remove', [row.name])}>
              {t('action.remove')}
            </Button>
          ) : null}
          {canApply('disable', row) ? (
            <Button icon="ban" onClick={() => openReview('disable', [row.name])}>
              {t('action.disable')}
            </Button>
          ) : null}
          {canApply('restore', row) ? (
            <Button variant="primary" icon="restore" onClick={() => openReview('restore', [row.name])}>
              {t('action.restore')}
            </Button>
          ) : null}
        </div>
      </div>
    </aside>
  );
}

export function Apps() {
  const d = ready.value;
  const all = packages.value;

  const shown = useComputed(() => {
    const list = packages.value ?? [];
    const q = search.value.trim().toLowerCase();
    const f = stateFilter.value;
    return list
      .filter((r) => (f === 'all' ? true : f === 'installed' ? r.state === 'installed' : r.state !== 'installed'))
      .filter((r) => !systemOnly.value || r.system)
      .filter((r) => levels.value.has(levelOf(r)))
      .filter((r) => !q || matches(r, q))
      .sort((a, b) => LEVEL_ORDER[levelOf(a)] - LEVEL_ORDER[levelOf(b)] || collator.value.compare(displayName(a), displayName(b)));
  });

  const levelCounts = useComputed(() => {
    const counts: Record<Level, number> = { recommended: 0, advanced: 0, expert: 0, unsafe: 0, unknown: 0 };
    for (const r of packages.value ?? []) {
      const f = stateFilter.value;
      if (f === 'installed' && r.state !== 'installed') continue;
      if (f === 'off' && r.state === 'installed') continue;
      counts[levelOf(r)]++;
    }
    return counts;
  });

  if (!d) {
    return (
      <div class="view-body">
        <PageHeader title={t('apps.title')} />
        <EmptyState icon="phone" title={t('apps.noPhone')} body={t('apps.noPhoneBody')}>
          <Button variant="primary" onClick={() => (page.value = 'phone')}>
            {t('apps.toPhone')}
          </Button>
        </EmptyState>
      </div>
    );
  }

  const rows = shown.value;
  const selected = selection.value;
  const selectedRows = (all ?? []).filter((r) => selected.has(r.name));
  const selectable = rows.filter((r) => canApply('remove', r) || canApply('disable', r) || canApply('restore', r));
  const allOn = selectable.length > 0 && selectable.every((r) => selected.has(r.name));
  const someOn = selectable.some((r) => selected.has(r.name));
  const panelRow = all?.find((r) => r.name === openPackage.value) ?? null;
  const recommended = rows.filter((r) => r.removal === 'recommended' && canApply('remove', r));
  const count = (a: Action) => selectedRows.filter((r) => canApply(a, r)).length;
  const query = search.value.trim();

  return (
    <div class="view">
      <div class="view-head">
        <PageHeader title={t('apps.title')} subtitle={t('apps.subtitle')}>
          <Button icon="refresh" onClick={() => void loadPackages()} disabled={loadingPackages.value}>
            {t('apps.reload')}
          </Button>
        </PageHeader>
        <div class="toolbar">
          <SearchBox value={search.value} onInput={(v) => (search.value = v)} placeholder={t('apps.searchPlaceholder')} />
          <Segmented
            label={t('apps.showLabel')}
            value={stateFilter.value}
            onChange={(v) => (stateFilter.value = v)}
            options={[
              { value: 'installed', label: t('apps.showInstalled') },
              { value: 'off', label: t('apps.showOff') },
              { value: 'all', label: t('apps.showAll') },
            ]}
          />
          <Switch checked={systemOnly.value} onChange={(v) => (systemOnly.value = v)} label={t('apps.systemOnly')} />
        </div>
        <div class="toolbar">
          <div class="chip-row" role="group" aria-label={t('apps.levelsLabel')}>
            {LEVELS.map((l) => (
              <Chip key={l} selected={levels.value.has(l)} onClick={() => toggleLevel(l)}>
                {t(l === 'unknown' ? 'safety.unknown' : `safety.${l as Removal}`)} <span class="muted">{formatNumber(levelCounts.value[l])}</span>
              </Chip>
            ))}
          </div>
          <span class="grow" />
          {recommended.length && stateFilter.value !== 'off' ? (
            <Button size="sm" variant="quiet" icon="check" onClick={() => setSelected(recommended.map((r) => r.name), true)}>
              {t('apps.selectRecommended', { count: recommended.length })}
            </Button>
          ) : null}
          <span class="toolbar-count muted">{t('apps.count', { count: rows.length })}</span>
        </div>
      </div>

      {packagesError.value ? (
        <div class="view-body">
          <Note icon="alert" tone="warn" title={t('apps.readFailed')}>
            {packagesError.value}
          </Note>
        </div>
      ) : !all ? (
        <div class="view-body">
          <p class="muted">{t('apps.reading')}</p>
        </div>
      ) : (
        <div class={`split ${panelRow ? 'has-panel' : ''}`}>
          <div class="table-wrap">
            {rows.length === 0 ? (
              <EmptyState icon="apps" title={query ? t('apps.emptySearch', { query }) : t('apps.empty')} />
            ) : (
              <div class="table pkg-table" role="table" aria-label={t('apps.title')}>
                <div class="thead" role="row">
                  <div class="th" role="columnheader">
                    <Checkbox
                      checked={allOn}
                      indeterminate={!allOn && someOn}
                      onChange={(on) => setSelected(selectable.map((r) => r.name), on)}
                      label={t('apps.selectShown')}
                      hideLabel
                      disabled={!selectable.length}
                    />
                  </div>
                  <div class="th" role="columnheader">{t('apps.colApp')}</div>
                  <div class="th" role="columnheader">{t('apps.colSafety')}</div>
                  <div class="th is-extra" role="columnheader">{t('apps.colState')}</div>
                  <div class="th is-extra" role="columnheader">{t('apps.colKind')}</div>
                </div>
                {rows.map((r) => {
                  const lock = lockReason(r);
                  const pick = canApply('remove', r) || canApply('disable', r) || canApply('restore', r);
                  return (
                    <div
                      key={r.name}
                      role="row"
                      class={`tr ${r.name === openPackage.value ? 'is-open' : ''} ${r.state !== 'installed' ? 'is-off' : ''}`}
                      tabIndex={0}
                      aria-selected={r.name === openPackage.value}
                      onClick={() => (openPackage.value = r.name === openPackage.value ? null : r.name)}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') {
                          e.preventDefault();
                          openPackage.value = r.name === openPackage.value ? null : r.name;
                        }
                      }}
                    >
                      <div class="td" role="cell">
                        {pick ? (
                          <Checkbox checked={selected.has(r.name)} onChange={(on) => setSelected([r.name], on)} label={t('apps.select', { name: displayName(r) })} hideLabel />
                        ) : lock ? (
                          <span title={lock} class="muted" aria-label={lock}>
                            <Icon name="lock" size={16} />
                          </span>
                        ) : null}
                      </div>
                      <div class="td" role="cell">
                        <span class="pkg-text">
                          <span class="pkg-label truncate">{displayName(r)}</span>
                          {r.label ? <span class="pkg-name mono muted truncate">{r.name}</span> : null}
                          {r.description ? <span class="pkg-desc muted truncate">{firstLine(r.description)}</span> : null}
                        </span>
                      </div>
                      <div class="td" role="cell">
                        <SafetyBadge removal={r.removal} />
                      </div>
                      <div class="td is-extra" role="cell">
                        <StateText state={r.state} />
                      </div>
                      <div class="td is-extra muted" role="cell">
                        {t(r.system ? 'apps.system' : 'apps.user')}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
            {selected.size ? (
              <div class="selection-bar" role="region" aria-label={t('apps.selectionLabel')}>
                <strong>{t('apps.selected', { count: selected.size })}</strong>
                <span class="grow" />
                <Button size="sm" variant="danger" icon="trash" disabled={!count('remove')} onClick={() => openReview('remove', selectedRows.map((r) => r.name))}>
                  {t('action.removeCount', { count: count('remove') })}
                </Button>
                <Button size="sm" icon="ban" disabled={!count('disable')} onClick={() => openReview('disable', selectedRows.map((r) => r.name))}>
                  {t('action.disableCount', { count: count('disable') })}
                </Button>
                <Button size="sm" icon="restore" disabled={!count('restore')} onClick={() => openReview('restore', selectedRows.map((r) => r.name))}>
                  {t('action.restoreCount', { count: count('restore') })}
                </Button>
                <Button size="sm" variant="quiet" onClick={clearSelection}>
                  {t('apps.clearSelection')}
                </Button>
              </div>
            ) : null}
          </div>
          {panelRow ? <Panel row={panelRow} onClose={() => (openPackage.value = null)} /> : null}
        </div>
      )}
    </div>
  );
}
