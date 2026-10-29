import { useEffect, useState } from 'react';
import { useIntegrationStore } from '../stores/integrationStore';
import { IconZap, IconBell, IconMonitor } from '../components/base/Icons';

export function IntegrationsPage() {
  const { config, loading, testResult, testError, obsRunning, fetchConfig, saveConfig, testWebhook, sendReport, startObs, stopObs } = useIntegrationStore();

  const [webhookUrl, setWebhookUrl] = useState('');
  const [username, setUsername] = useState('OverDrive');
  const [notifyAlert, setNotifyAlert] = useState(true);
  const [notifyHighTemp, setNotifyHighTemp] = useState(true);
  const [highTempThreshold, setHighTempThreshold] = useState(80);
  const [obsPort, setObsPort] = useState(9877);
  const [obsRefreshRate, setObsRefreshRate] = useState(1000);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    fetchConfig();
  }, [fetchConfig]);

  useEffect(() => {
    if (config) {
      setWebhookUrl(config.discord_webhook_url);
      setUsername(config.discord_username);
      setNotifyAlert(config.discord_notify_on_alert);
      setNotifyHighTemp(config.discord_notify_on_high_temp);
      setHighTempThreshold(config.discord_high_temp_threshold);
      setObsPort(config.obs_port);
      setObsRefreshRate(config.obs_refresh_rate_ms);
    }
  }, [config]);

  const handleSave = async () => {
    setSaving(true);
    await saveConfig({
      discord_webhook_url: webhookUrl,
      discord_username: username,
      discord_avatar_url: '',
      discord_notify_on_alert: notifyAlert,
      discord_notify_on_high_temp: notifyHighTemp,
      discord_high_temp_threshold: highTempThreshold,
      obs_enabled: obsRunning,
      obs_port: obsPort,
      obs_refresh_rate_ms: obsRefreshRate,
    });
    setSaving(false);
  };

  const handleTestWebhook = () => {
    testWebhook(webhookUrl);
  };

  const handleSendReport = () => {
    sendReport(webhookUrl);
  };

  const handleToggleObs = () => {
    if (obsRunning) {
      stopObs();
    } else {
      startObs(obsPort);
    }
  };

  if (loading && !config) {
    return (
      <div className="flex h-64 items-center justify-center">
        <div className="ac-spinner" style={{ width: 32, height: 32, borderWidth: 3 }} />
      </div>
    );
  }

  return (
    <div className="ac-page ac-page--wide">
      <div className="ac-page-header">
        <div className="ac-page-header__left">
          <h2 className="ac-page-header__title">Integrations</h2>
          <p className="ac-page-header__desc">Connect OverDrive to external services</p>
        </div>
      </div>

      <div className="ac-grid-2">
        <div className="ac-page-card">
          <div className="ac-page-card__header">
            <div className="ac-page-card__title">
              <div style={{ width: 20, height: 20, display: 'flex', alignItems: 'center', justifyContent: 'center', borderRadius: 4, background: 'rgba(88,101,242,0.2)', color: '#5865F2' }}>
                <IconBell className="size-3" />
              </div>
              Discord Webhook
            </div>
          </div>
          <div className="ac-page-card__body" style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div>
              <label className="ac-label">Webhook URL</label>
              <input
                type="text"
                value={webhookUrl}
                onChange={(e) => setWebhookUrl(e.target.value)}
                placeholder="https://discord.com/api/webhooks/..."
                className="ac-input ac-input--wide"
              />
            </div>

            <div>
              <label className="ac-label">Bot Username</label>
              <input
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                className="ac-input ac-input--wide"
              />
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              <label className="ac-checkbox" onClick={() => setNotifyAlert(!notifyAlert)}>
                <span className={`ac-checkbox__box ${notifyAlert ? 'ac-checkbox__box--checked' : ''}`} />
                <span className="ac-checkbox__label">Notify on alerts</span>
              </label>

              <label className="ac-checkbox" onClick={() => setNotifyHighTemp(!notifyHighTemp)}>
                <span className={`ac-checkbox__box ${notifyHighTemp ? 'ac-checkbox__box--checked' : ''}`} />
                <span className="ac-checkbox__label">Notify on high temperature</span>
              </label>

              {notifyHighTemp && (
                <div style={{ marginLeft: 24, display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span style={{ color: 'var(--ac-text-secondary)', fontSize: 12 }}>Threshold:</span>
                  <input
                    type="number"
                    value={highTempThreshold}
                    onChange={(e) => setHighTempThreshold(Number(e.target.value))}
                    className="ac-input"
                    style={{ width: 80, textAlign: 'center' }}
                  />
                  <span style={{ color: 'var(--ac-text-secondary)', fontSize: 12 }}>°C</span>
                </div>
              )}
            </div>

            <div style={{ display: 'flex', gap: 8 }}>
              <button onClick={handleTestWebhook} className="ac-btn ac-btn--secondary ac-btn--sm">
                Test Webhook
              </button>
              <button onClick={handleSendReport} className="ac-btn ac-btn--secondary ac-btn--sm">
                Send Test Report
              </button>
            </div>

            {testResult && (
              <div className="ac-banner ac-banner--success" style={{ fontSize: 11 }}>
                {testResult}
              </div>
            )}
            {testError && (
              <div className="ac-banner ac-banner--error" style={{ fontSize: 11 }}>
                {testError}
              </div>
            )}
          </div>
        </div>

        <div className="ac-page-card">
          <div className="ac-page-card__header">
            <div className="ac-page-card__title">
              <div className="ac-page-card__title-icon" style={{ width: 20, height: 20, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <IconMonitor className="size-3.5" />
              </div>
              OBS Browser Source
            </div>
          </div>
          <div className="ac-page-card__body" style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div>
              <label className="ac-label">HTTP Port</label>
              <input
                type="number"
                value={obsPort}
                onChange={(e) => setObsPort(Number(e.target.value))}
                className="ac-input"
                style={{ width: 96 }}
              />
            </div>

            <div>
              <label className="ac-label">Refresh Rate</label>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <input
                  type="number"
                  value={obsRefreshRate}
                  onChange={(e) => setObsRefreshRate(Number(e.target.value))}
                  className="ac-input"
                  style={{ width: 96 }}
                />
                <span style={{ color: 'var(--ac-text-secondary)', fontSize: 12 }}>ms</span>
              </div>
            </div>

            <button
              onClick={handleToggleObs}
              className={`ac-btn ac-btn--${obsRunning ? 'danger' : 'primary'}`}
            >
              {obsRunning ? 'Stop OBS Source' : 'Start OBS Source'}
            </button>

            {obsRunning && (
              <div className="ac-banner ac-banner--success" style={{ fontSize: 11 }}>
                OBS Browser Source running on port {obsPort}
                <br />
                Add a Browser Source in OBS pointing to:
                <span style={{ display: 'block', marginTop: 4, fontFamily: 'var(--ac-font-mono)', color: 'var(--ac-text-primary)' }}>
                  http://localhost:{obsPort}
                </span>
              </div>
            )}
          </div>
        </div>
      </div>

      <div className="ac-page-card">
        <div className="ac-page-card__header">
          <div className="ac-page-card__title">Steam Integration</div>
        </div>
        <div className="ac-page-card__body" style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          <p style={{ color: 'var(--ac-text-secondary)', fontSize: 12, lineHeight: 1.4 }}>
            Steam integration automatically detects games you launch through Steam and applies GPU profiles.
            Game detection is already active — configure per-game profiles in the Profiles section.
          </p>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '8px 12px', borderRadius: 'var(--ac-radius-panel)', background: 'var(--ac-bg-panel)', border: '1px solid var(--ac-border-subtle)', color: 'var(--ac-text-secondary)', fontSize: 12 }}>
            <IconZap className="ac-page-card__title-icon" />
            Steam game detection is always active
          </div>
        </div>
      </div>

      <div className="ac-page-card">
        <div className="ac-page-card__header">
          <div className="ac-page-card__title">Custom REST API</div>
        </div>
        <div className="ac-page-card__body" style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          <p style={{ color: 'var(--ac-text-secondary)', fontSize: 12, lineHeight: 1.4 }}>
            The Remote Monitoring server exposes a REST API for custom integrations.
            Enable it in the Remote section to query GPU metrics from your own applications.
          </p>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '8px 12px', borderRadius: 'var(--ac-radius-panel)', background: 'var(--ac-bg-panel)', border: '1px solid var(--ac-border-subtle)', color: 'var(--ac-text-secondary)', fontSize: 12 }}>
            <IconZap className="ac-page-card__title-icon" />
            Configure in Settings → Remote Monitoring
          </div>
        </div>
      </div>

      <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
        <button onClick={handleSave} disabled={saving} className="ac-btn ac-btn--primary">
          {saving ? 'Saving...' : 'Save Settings'}
        </button>
      </div>
    </div>
  );
}
