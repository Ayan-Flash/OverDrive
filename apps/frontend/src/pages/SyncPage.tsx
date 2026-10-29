import { useEffect, useState } from 'react';
import { useSyncStore } from '../stores/syncStore';
import { IconRefresh, IconGlobe, IconCheck } from '../components/base/Icons';

function formatDate(ts: number | null) {
  if (!ts) return 'Never';
  return new Date(ts * 1000).toLocaleString();
}

export function SyncPage() {
  const { status, lastResult, loading, error, fetchStatus, register, unregister, sync, start, stop, updateSettings, clearError } = useSyncStore();

  const [serverUrl, setServerUrl] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [interval, setInterval] = useState(300);
  const [syncProfiles, setSyncProfiles] = useState(true);
  const [syncReports, setSyncReports] = useState(false);
  const [syncPolicies, setSyncPolicies] = useState(true);
  const [serverRunning, setServerRunning] = useState(false);

  useEffect(() => {
    fetchStatus();
  }, [fetchStatus]);

  useEffect(() => {
    if (status) {
      setServerUrl(status.server_url);
    }
  }, [status]);

  const handleRegister = () => {
    register(serverUrl, apiKey);
  };

  const handleUnregister = () => {
    unregister();
    setApiKey('');
  };

  const handleSaveSettings = () => {
    updateSettings(serverUrl, apiKey, interval, syncProfiles, syncReports, syncPolicies);
  };

  const toggleClient = () => {
    if (status?.device_registered) {
      stop();
    } else {
      start();
    }
  };

  return (
    <div className="ac-page">
      <div className="ac-page-header">
        <div className="ac-page-header__left">
          <div className="ac-page-header__title">Cloud Sync</div>
          <div className="ac-page-header__desc">Synchronize profiles, reports, and settings with a cloud server</div>
        </div>
      </div>

      {error && (
        <div className="ac-banner ac-banner--error">
          {error}
          <button onClick={clearError} className="ac-banner__close">&times;</button>
        </div>
      )}

      {lastResult && (
        <div className="ac-banner ac-banner--success">
          Sync complete: {lastResult.profiles_synced} profiles, {lastResult.reports_synced} reports
        </div>
      )}

      <div className="ac-grid-2" style={{gap: 24}}>
        <div className="ac-page-card">
          <div className="ac-page-card__body" style={{display: 'flex', flexDirection: 'column', gap: 16}}>
            <div style={{display: 'flex', alignItems: 'center', gap: 12}}>
              <div style={{display: 'flex', width: 40, height: 40, alignItems: 'center', justifyContent: 'center', borderRadius: 8, background: 'rgba(0,170,220,0.2)'}}>
                <IconGlobe style={{width: 20, height: 20, color: 'var(--ac-accent-cyan-bright)'}} />
              </div>
              <div>
                <h3 style={{color: 'var(--ac-text-primary)', fontSize: 13, fontWeight: 600}}>Cloud Server</h3>
                <p style={{color: 'var(--ac-text-secondary)', fontSize: 12}}>Connect to your sync server</p>
              </div>
            </div>

            <div style={{display: 'flex', flexDirection: 'column', gap: 12}}>
              <div>
                <label className="ac-label">Server URL</label>
                <input
                  type="text"
                  value={serverUrl}
                  onChange={(e) => setServerUrl(e.target.value)}
                  placeholder="https://sync.example.com"
                  className="ac-input ac-input--wide"
                />
              </div>

              {!status?.device_registered && (
                <div>
                  <label className="ac-label">API Key</label>
                  <input
                    type="password"
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    className="ac-input ac-input--wide"
                  />
                </div>
              )}

              <div style={{display: 'flex', gap: 8}}>
                {!status?.device_registered ? (
                  <button onClick={handleRegister} className="ac-btn ac-btn--primary ac-btn--sm">
                    Register Device
                  </button>
                ) : (
                  <button onClick={handleUnregister} className="ac-btn ac-btn--danger ac-btn--sm">
                    Unregister
                  </button>
                )}
                <button onClick={handleSaveSettings} className="ac-btn ac-btn--secondary ac-btn--sm">
                  Save Settings
                </button>
              </div>

              {status?.device_registered && (
                <div style={{display: 'flex', alignItems: 'center', gap: 8, borderRadius: 8, border: '1px solid rgba(52,211,153,0.3)', background: 'rgba(52,211,153,0.1)', padding: '8px 12px', color: '#34d399', fontSize: 12}}>
                  <IconCheck style={{width: 14, height: 14}} />
                  Device registered
                </div>
              )}
            </div>
          </div>
        </div>

        <div className="ac-page-card">
          <div className="ac-page-card__body" style={{display: 'flex', flexDirection: 'column', gap: 16}}>
            <div style={{display: 'flex', alignItems: 'center', gap: 12}}>
              <div style={{display: 'flex', width: 40, height: 40, alignItems: 'center', justifyContent: 'center', borderRadius: 8, background: 'rgba(59,130,246,0.2)'}}>
                <IconRefresh style={{width: 20, height: 20, color: '#60a5fa'}} />
              </div>
              <div>
                <h3 style={{color: 'var(--ac-text-primary)', fontSize: 13, fontWeight: 600}}>Sync Status</h3>
                <p style={{color: 'var(--ac-text-secondary)', fontSize: 12}}>Last sync: {formatDate(status?.last_sync_at ?? null)}</p>
              </div>
            </div>

            <div style={{display: 'flex', flexDirection: 'column', gap: 12}}>
              <div className="ac-grid-2" style={{gap: 12}}>
                <div style={{background: 'var(--ac-bg-input)', borderRadius: 8, padding: '8px 12px'}}>
                  <p style={{color: 'var(--ac-text-secondary)', fontSize: 12}}>Server</p>
                  <p style={{color: 'var(--ac-text-primary)', fontSize: 13, fontFamily: 'var(--ac-font-mono)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap'}}>
                    {status?.server_url || 'Not configured'}
                  </p>
                </div>
                <div style={{background: 'var(--ac-bg-input)', borderRadius: 8, padding: '8px 12px'}}>
                  <p style={{color: 'var(--ac-text-secondary)', fontSize: 12}}>Registered</p>
                  <p style={{color: 'var(--ac-text-primary)', fontSize: 13}}>{status?.device_registered ? 'Yes' : 'No'}</p>
                </div>
              </div>

              <div style={{display: 'flex', gap: 8}}>
                <button onClick={sync} disabled={loading || !status?.device_registered} className="ac-btn ac-btn--primary ac-btn--sm">
                  {loading ? 'Syncing...' : 'Sync Now'}
                </button>
                <button onClick={toggleClient} className={`ac-btn ac-btn--${status?.device_registered ? 'danger' : 'primary'} ac-btn--sm`}>
                  {status?.device_registered ? 'Stop Sync Client' : 'Start Sync Client'}
                </button>
              </div>

              <div style={{display: 'flex', flexDirection: 'column', gap: 8}}>
                <p style={{color: 'var(--ac-text-secondary)', fontSize: 12, fontWeight: 500}}>Sync Settings</p>
                <div>
                  <label className="ac-label">Sync Interval</label>
                  <div style={{display: 'flex', alignItems: 'center', gap: 8}}>
                    <input
                      type="number"
                      value={interval}
                      onChange={(e) => setInterval(Number(e.target.value))}
                      min={30}
                      className="ac-input ac-input--sm"
                      style={{width: 96}}
                    />
                    <span style={{color: 'var(--ac-text-secondary)', fontSize: 12}}>seconds</span>
                  </div>
                </div>
                <label className="ac-checkbox">
                  <input type="checkbox" checked={syncProfiles} onChange={(e) => setSyncProfiles(e.target.checked)} style={{accentColor: 'var(--ac-accent-cyan)', width: 14, height: 14}} />
                  <span className="ac-checkbox__label">Sync profiles</span>
                </label>
                <label className="ac-checkbox">
                  <input type="checkbox" checked={syncReports} onChange={(e) => setSyncReports(e.target.checked)} style={{accentColor: 'var(--ac-accent-cyan)', width: 14, height: 14}} />
                  <span className="ac-checkbox__label">Sync reports</span>
                </label>
                <label className="ac-checkbox">
                  <input type="checkbox" checked={syncPolicies} onChange={(e) => setSyncPolicies(e.target.checked)} style={{accentColor: 'var(--ac-accent-cyan)', width: 14, height: 14}} />
                  <span className="ac-checkbox__label">Sync policies</span>
                </label>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div className="ac-page-card">
        <div className="ac-page-card__body" style={{display: 'flex', flexDirection: 'column', gap: 12}}>
          <div className="ac-page-header">
            <h3 style={{color: 'var(--ac-text-primary)', fontSize: 13, fontWeight: 600}}>Local Sync Server</h3>
            <button
              onClick={async () => {
                const { isSyncServerRunning, startSyncServer, stopSyncServer } = await import('../services/syncService');
                const running = await isSyncServerRunning();
                if (running) {
                  await stopSyncServer();
                  setServerRunning(false);
                } else {
                  await startSyncServer();
                  setServerRunning(true);
                }
              }}
              className={`ac-btn ac-btn--${serverRunning ? 'danger' : 'primary'} ac-btn--sm`}
            >
              {serverRunning ? 'Stop Server' : 'Start Server'}
            </button>
          </div>
          <p style={{color: 'var(--ac-text-secondary)', fontSize: 12}}>
            Run a local sync server to receive sync requests from other OverDrive instances.
            Other instances can sync to <code style={{color: 'var(--ac-accent-cyan-bright)'}}>http://localhost:9878/api/sync</code>
          </p>
          {serverRunning && (
            <div style={{borderRadius: 8, border: '1px solid rgba(52,211,153,0.3)', background: 'rgba(52,211,153,0.1)', padding: '8px 12px', color: '#34d399', fontSize: 12}}>
              Sync server running on port 9878
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
