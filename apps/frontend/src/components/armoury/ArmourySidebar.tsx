import { navIcons } from '../base/Icons';
import { useUiStore } from '../../stores';

type PageId = Parameters<ReturnType<typeof useUiStore.getState>['navigateTo']>[0];

const groups: { label: string; items: { id: PageId; label: string }[] }[] = [
  { label: 'Workspace', items: [{ id: 'dashboard', label: 'Overview' }, { id: 'reports', label: 'Performance Lab' }] },
  { label: 'Hardware', items: [{ id: 'control', label: 'Control' }, { id: 'profiles', label: 'Profiles' }, { id: 'marketplace', label: 'Profile Marketplace' }] },
  { label: 'Intelligence', items: [{ id: 'ai', label: 'AI Insights' }, { id: 'automation', label: 'Automation' }] },
  { label: 'Gaming', items: [{ id: 'overlay', label: 'Overlay' }] },
  { label: 'Connectivity', items: [{ id: 'remote', label: 'Remote Monitoring' }, { id: 'integrations', label: 'Integrations' }, { id: 'sync', label: 'Cloud Sync' }] },
  { label: 'System', items: [{ id: 'alerts', label: 'Alerts' }, { id: 'backup', label: 'Backup & Restore' }, { id: 'enterprise', label: 'Enterprise' }, { id: 'settings', label: 'Settings' }] },
];

export function ArmourySidebar() {
  const currentPage = useUiStore((state) => state.currentPage);
  const navigateTo = useUiStore((state) => state.navigateTo);

  return (
    <aside className="ac-sidebar" aria-label="Main navigation">
      <div className="ac-sidebar__brand">
        <span className="ac-sidebar__brand-mark" aria-hidden="true">
          <svg viewBox="0 0 32 32" fill="none">
            <path d="M16 2.8 28 9.5v13L16 29.2 4 22.5v-13L16 2.8Z" />
            <path d="m16 8.5 7 4v7l-7 4-7-4v-7l7-4Z" />
          </svg>
        </span>
        <span className="ac-sidebar__brand-copy">
          <strong>Overdrive</strong>
          <small>GPU CONTROL</small>
        </span>
      </div>

      <nav className="ac-sidebar__nav">
        {groups.map((group) => (
          <div className="ac-sidebar__group" key={group.label}>
            <div className="ac-sidebar__group-label">{group.label}</div>
            {group.items.map(({ id, label }) => {
              const Icon = navIcons[id as keyof typeof navIcons];
              const active = currentPage === id;
              return (
                <button
                  className={`ac-sidebar__item ${active ? 'ac-sidebar__item--active' : ''}`}
                  key={id}
                  onClick={() => navigateTo(id)}
                  aria-current={active ? 'page' : undefined}
                  title={label}
                >
                  <Icon className="ac-sidebar__icon" size={18} />
                  <span>{label}</span>
                </button>
              );
            })}
          </div>
        ))}
      </nav>

      <div className="ac-sidebar__footer">
        <span className="ac-sidebar__status-dot" />
        <span>Local system</span>
      </div>
    </aside>
  );
}

