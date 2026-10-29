import { useAlertStore } from '../../stores';

export function AlertPanel() {
  const { alerts, unacknowledgedCount, acknowledgeAlert, clearAlerts } = useAlertStore();

  const severityStyle = (severity: string) => {
    switch (severity) {
      case 'Critical': return 'border-l-red-500 bg-red-500/5';
      case 'Warning': return 'border-l-amber-500 bg-amber-500/5';
      default: return 'border-l-sky-500 bg-sky-500/5';
    }
  };

  if (alerts.length === 0) {
    return (
      <div className="card p-4 text-center">
        <span className="text-text-muted text-xs">No alerts</span>
      </div>
    );
  }

  return (
    <div className="card flex flex-col gap-2 p-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-xs font-semibold uppercase tracking-wide text-text-primary">Alerts</span>
          {unacknowledgedCount > 0 && (
            <span className="text-accent-bright inline-flex h-[18px] min-w-[18px] items-center justify-center rounded-full bg-accent/20 px-1 text-[10px] font-bold">
              {unacknowledgedCount}
            </span>
          )}
        </div>
        <button onClick={clearAlerts} className="text-text-muted text-[10px] uppercase tracking-wider transition-colors hover:text-text-secondary">
          Clear
        </button>
      </div>

      <div className="flex max-h-40 flex-col gap-1 overflow-y-auto">
        {[...alerts].reverse().slice(0, 20).map((alert) => (
          <div
            key={alert.id}
            className={`flex items-start gap-2 rounded-lg border-l-2 px-3 py-2 text-xs ${severityStyle(alert.severity)} ${
              alert.acknowledged ? 'opacity-40' : ''
            }`}
          >
            <span className="flex-1 leading-relaxed text-text-secondary">{alert.message}</span>
            <div className="flex flex-shrink-0 items-center gap-1.5">
              <span className="text-[10px] text-text-dim">{new Date(alert.timestamp).toLocaleTimeString()}</span>
              {!alert.acknowledged && (
                <button
                  onClick={() => acknowledgeAlert(alert.id)}
                  className="text-text-muted flex size-4 items-center justify-center rounded bg-gpu-700 transition-colors hover:bg-gpu-600 hover:text-text-secondary"
                  title="Acknowledge"
                >
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                    <polyline points="20 6 9 17 4 12" />
                  </svg>
                </button>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
