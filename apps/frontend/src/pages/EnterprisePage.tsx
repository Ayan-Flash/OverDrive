import { useEffect, useState } from 'react';
import { useEnterpriseStore } from '../stores/enterpriseStore';
import { IconSettings, IconShield, IconGlobe, IconX } from '../components/base/Icons';

function formatDate(ts: number) {
  return new Date(ts * 1000).toLocaleString();
}

export function EnterprisePage() {
  const { config, policies, loading, error, fetchConfig, saveConfig, fetchPolicies, createPolicy, deletePolicy, togglePolicy, clearError } = useEnterpriseStore();

  const [appName, setAppName] = useState('OverDrive');
  const [primaryColor, setPrimaryColor] = useState('#f04747');
  const [showBranding, setShowBranding] = useState(true);
  const [policiesEnabled, setPoliciesEnabled] = useState(true);
  const [enforcementLevel, setEnforcementLevel] = useState<'Recommended' | 'Enforced' | 'Strict'>('Recommended');
  const [centralEnabled, setCentralEnabled] = useState(false);
  const [serverUrl, setServerUrl] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [syncInterval, setSyncInterval] = useState(300);
  const [machineName, setMachineName] = useState('');
  const [syncPolicies, setSyncPolicies] = useState(true);
  const [syncProfiles, setSyncProfiles] = useState(true);

  const [newPolicyName, setNewPolicyName] = useState('');
  const [newPolicyDesc, setNewPolicyDesc] = useState('');
  const [newPolicyTarget, setNewPolicyTarget] = useState<'all' | 'vendor'>('all');
  const [newPolicyVendor, setNewPolicyVendor] = useState('nvidia');

  const [saving, setSaving] = useState(false);
  const [activeTab, setActiveTab] = useState<'branding' | 'policies' | 'centralized'>('branding');

  useEffect(() => {
    fetchConfig();
    fetchPolicies();
  }, [fetchConfig, fetchPolicies]);

  useEffect(() => {
    if (config) {
      setAppName(config.branding.app_name);
      setPrimaryColor(config.branding.primary_color);
      setShowBranding(config.branding.show_branding);
      setPoliciesEnabled(config.policies_enabled);
      setEnforcementLevel(config.enforcement_level);
      setCentralEnabled(config.centralized.enabled);
      setServerUrl(config.centralized.server_url);
      setApiKey(config.centralized.api_key);
      setSyncInterval(config.centralized.sync_interval_secs);
      setMachineName(config.centralized.machine_name);
      setSyncPolicies(config.centralized.sync_policies);
      setSyncProfiles(config.centralized.sync_profiles);
    }
  }, [config]);

  const handleSave = async () => {
    if (!config) return;
    setSaving(true);
    await saveConfig({
      branding: {
        ...config.branding,
        app_name: appName,
        primary_color: primaryColor,
        show_branding: showBranding,
      },
      centralized: {
        ...config.centralized,
        enabled: centralEnabled,
        server_url: serverUrl,
        api_key: apiKey,
        sync_interval_secs: syncInterval,
        machine_name: machineName,
        sync_policies: syncPolicies,
        sync_profiles: syncProfiles,
      },
      policies_enabled: policiesEnabled,
      enforcement_level: enforcementLevel,
    });
    setSaving(false);
  };

  const handleCreatePolicy = async () => {
    if (!newPolicyName.trim()) return;
    const target = newPolicyTarget === 'all'
      ? { AllGpus: {} }
      : { ByVendor: newPolicyVendor };
    await createPolicy(newPolicyName, newPolicyDesc, target);
    setNewPolicyName('');
    setNewPolicyDesc('');
  };

  if (loading && !config) {
    return (
      <div style={{display: 'flex', height: 256, alignItems: 'center', justifyContent: 'center'}}>
        <div className="ac-spinner" style={{width: 32, height: 32}} />
      </div>
    );
  }

  const tabs = [
    { id: 'branding' as const, label: 'Branding', icon: IconSettings },
    { id: 'policies' as const, label: 'Group Policies', icon: IconShield },
    { id: 'centralized' as const, label: 'Centralized Management', icon: IconGlobe },
  ];

  return (
    <div className="ac-page">
      <div className="ac-page-header">
        <div className="ac-page-header__left">
          <div className="ac-page-header__title">Enterprise</div>
          <div className="ac-page-header__desc">Group policies, branding, and centralized management</div>
        </div>
      </div>

      {error && (
        <div className="ac-banner ac-banner--error">
          {error}
          <button onClick={clearError} className="ac-banner__close">&times;</button>
        </div>
      )}

      <div className="ac-tabs">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`ac-tab ${activeTab === tab.id ? 'ac-tab--active' : ''}`}
            style={{display: 'flex', alignItems: 'center', gap: 8}}
          >
            <tab.icon style={{width: 16, height: 16}} />
            {tab.label}
          </button>
        ))}
      </div>

      {activeTab === 'branding' && (
        <div className="ac-page-card">
          <div className="ac-page-card__body" style={{display: 'flex', flexDirection: 'column', gap: 16}}>
            <h3 style={{color: 'var(--ac-text-primary)', fontSize: 13, fontWeight: 600}}>Application Branding</h3>

            <div style={{display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16}}>
              <div>
                <label className="ac-label">Application Name</label>
                <input
                  type="text"
                  value={appName}
                  onChange={(e) => setAppName(e.target.value)}
                  className="ac-input ac-input--wide"
                />
              </div>

              <div>
                <label className="ac-label">Primary Color</label>
                <div style={{display: 'flex', alignItems: 'center', gap: 8}}>
                  <input
                    type="color"
                    value={primaryColor}
                    onChange={(e) => setPrimaryColor(e.target.value)}
                    style={{width: 40, height: 40, cursor: 'pointer', borderRadius: 8, border: '1px solid var(--ac-border-subtle)', background: 'var(--ac-bg-input)', padding: 0, outline: 'none'}}
                  />
                  <input
                    type="text"
                    value={primaryColor}
                    onChange={(e) => setPrimaryColor(e.target.value)}
                    className="ac-input ac-input--wide"
                    style={{fontFamily: 'var(--ac-font-mono)'}}
                  />
                </div>
              </div>
            </div>

            <label className="ac-checkbox">
              <input
                type="checkbox"
                checked={showBranding}
                onChange={(e) => setShowBranding(e.target.checked)}
                style={{accentColor: 'var(--ac-accent-cyan)', width: 14, height: 14}}
              />
              <span className="ac-checkbox__label">Show branding in UI</span>
            </label>

            {showBranding && (
              <div style={{background: 'var(--ac-bg-input)', borderRadius: 8, padding: 12, color: 'var(--ac-text-secondary)', fontSize: 12}}>
                <p style={{marginBottom: 4}}>Preview:</p>
                <div style={{display: 'flex', alignItems: 'center', gap: 12}}>
                  <div style={{width: 32, height: 32, borderRadius: 8, background: primaryColor}} />
                  <span style={{fontWeight: 600, color: primaryColor}}>{appName}</span>
                  <span style={{color: 'var(--ac-text-muted)'}}>Enterprise Edition</span>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {activeTab === 'policies' && (
        <div style={{display: 'flex', flexDirection: 'column', gap: 16}}>
          <div className="ac-page-card">
            <div className="ac-page-card__body" style={{display: 'flex', flexDirection: 'column', gap: 16}}>
              <div className="ac-page-header">
                <h3 style={{color: 'var(--ac-text-primary)', fontSize: 13, fontWeight: 600}}>Group Policies</h3>
                <div className="ac-page-header__right">
                  <label className="ac-checkbox">
                    <input
                      type="checkbox"
                      checked={policiesEnabled}
                      onChange={(e) => setPoliciesEnabled(e.target.checked)}
                      style={{accentColor: 'var(--ac-accent-cyan)', width: 14, height: 14}}
                    />
                    <span className="ac-checkbox__label">Enable policies</span>
                  </label>
                </div>
              </div>

              <div style={{display: 'flex', alignItems: 'center', gap: 8}}>
                <select
                  value={enforcementLevel}
                  onChange={(e) => setEnforcementLevel(e.target.value as 'Recommended' | 'Enforced' | 'Strict')}
                  className="ac-input ac-select ac-input--sm"
                >
                  <option value="Recommended">Recommended</option>
                  <option value="Enforced">Enforced</option>
                  <option value="Strict">Strict</option>
                </select>
                <span style={{color: 'var(--ac-text-secondary)', fontSize: 12}}>
                  {enforcementLevel === 'Recommended' && 'Users can override policy settings'}
                  {enforcementLevel === 'Enforced' && 'Policy settings are locked for users'}
                  {enforcementLevel === 'Strict' && 'Policies enforced with compliance logging'}
                </span>
              </div>

              <div style={{display: 'flex', flexDirection: 'column', gap: 8}}>
                <h4 className="ac-subtitle" style={{marginBottom: 0}}>Create New Policy</h4>
                <div style={{display: 'flex', flexWrap: 'wrap', gap: 8}}>
                  <input
                    type="text"
                    value={newPolicyName}
                    onChange={(e) => setNewPolicyName(e.target.value)}
                    placeholder="Policy name"
                    className="ac-input ac-input--sm"
                    style={{width: 192}}
                  />
                  <input
                    type="text"
                    value={newPolicyDesc}
                    onChange={(e) => setNewPolicyDesc(e.target.value)}
                    placeholder="Description"
                    className="ac-input ac-input--sm"
                    style={{width: 256}}
                  />
                  <select
                    value={newPolicyTarget}
                    onChange={(e) => setNewPolicyTarget(e.target.value as 'all' | 'vendor')}
                    className="ac-input ac-select ac-input--sm"
                  >
                    <option value="all">All GPUs</option>
                    <option value="vendor">By Vendor</option>
                  </select>
                  {newPolicyTarget === 'vendor' && (
                    <select
                      value={newPolicyVendor}
                      onChange={(e) => setNewPolicyVendor(e.target.value)}
                      className="ac-input ac-select ac-input--sm"
                    >
                      <option value="nvidia">NVIDIA</option>
                      <option value="amd">AMD</option>
                      <option value="intel">Intel</option>
                    </select>
                  )}
                  <button onClick={handleCreatePolicy} className="ac-btn ac-btn--primary ac-btn--sm">
                    Create
                  </button>
                </div>
              </div>
            </div>
          </div>

          {policies.length === 0 ? (
            <div className="ac-empty">
              <IconShield className="ac-empty__icon" />
              <div className="ac-empty__text">No group policies defined</div>
            </div>
          ) : (
            <div style={{display: 'flex', flexDirection: 'column', gap: 8}}>
              {policies.map((p) => (
                <div key={p.id} className="ac-page-card">
                  <div className="ac-page-card__body">
                    <div className="ac-page-header">
                      <div className="ac-page-header__left" style={{display: 'flex', flexDirection: 'row', alignItems: 'center', gap: 12, minWidth: 0}}>
                        <div className="ac-toggle" onClick={() => togglePolicy(p.id, !p.enabled)}>
                          <div className={`ac-toggle__track ${p.enabled ? 'ac-toggle__track--on' : ''}`}>
                            <div className="ac-toggle__thumb" />
                          </div>
                        </div>
                        <div style={{minWidth: 0}}>
                          <p style={{color: 'var(--ac-text-primary)', fontSize: 13, fontWeight: 500}}>{p.name}</p>
                          <p style={{color: 'var(--ac-text-secondary)', fontSize: 12}}>{p.description}</p>
                          <p style={{color: 'var(--ac-text-muted)', fontSize: 12, marginTop: 2}}>
                            Priority {p.priority} &middot; {formatDate(p.created_at)}
                          </p>
                        </div>
                      </div>
                      <button
                        onClick={() => deletePolicy(p.id)}
                        className="ac-btn ac-btn--ghost ac-btn--icon"
                        style={{color: 'rgba(255,68,68,0.6)'}}
                      >
                        <IconX style={{width: 14, height: 14}} />
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {activeTab === 'centralized' && (
        <div className="ac-page-card">
          <div className="ac-page-card__body" style={{display: 'flex', flexDirection: 'column', gap: 16}}>
            <h3 style={{color: 'var(--ac-text-primary)', fontSize: 13, fontWeight: 600}}>Centralized Management Server</h3>

            <label className="ac-checkbox">
              <input
                type="checkbox"
                checked={centralEnabled}
                onChange={(e) => setCentralEnabled(e.target.checked)}
                style={{accentColor: 'var(--ac-accent-cyan)', width: 14, height: 14}}
              />
              <span className="ac-checkbox__label">Enable centralized management</span>
            </label>

            {centralEnabled && (
              <div style={{borderLeft: '2px solid rgba(0,170,220,0.3)', paddingLeft: 24, display: 'flex', flexDirection: 'column', gap: 12}}>
                <div>
                  <label className="ac-label">Server URL</label>
                  <input
                    type="text"
                    value={serverUrl}
                    onChange={(e) => setServerUrl(e.target.value)}
                    placeholder="https://mgmt.example.com"
                    className="ac-input ac-input--wide"
                    style={{maxWidth: 400}}
                  />
                </div>

                <div>
                  <label className="ac-label">API Key</label>
                  <input
                    type="password"
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    className="ac-input ac-input--wide"
                    style={{maxWidth: 400}}
                  />
                </div>

                <div style={{display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16}}>
                  <div>
                    <label className="ac-label">Machine Name</label>
                    <input
                      type="text"
                      value={machineName}
                      onChange={(e) => setMachineName(e.target.value)}
                      className="ac-input ac-input--wide"
                    />
                  </div>

                  <div>
                    <label className="ac-label">Sync Interval</label>
                    <div style={{display: 'flex', alignItems: 'center', gap: 8}}>
                      <input
                        type="number"
                        value={syncInterval}
                        onChange={(e) => setSyncInterval(Number(e.target.value))}
                        min={30}
                        className="ac-input ac-input--sm"
                        style={{width: 96}}
                      />
                      <span style={{color: 'var(--ac-text-secondary)', fontSize: 12}}>seconds</span>
                    </div>
                  </div>
                </div>

                <div style={{display: 'flex', flexDirection: 'column', gap: 8}}>
                  <p style={{color: 'var(--ac-text-secondary)', fontSize: 12, fontWeight: 500}}>Sync Settings</p>
                  <label className="ac-checkbox">
                    <input type="checkbox" checked={syncPolicies} onChange={(e) => setSyncPolicies(e.target.checked)} style={{accentColor: 'var(--ac-accent-cyan)', width: 14, height: 14}} />
                    <span className="ac-checkbox__label">Sync group policies</span>
                  </label>
                  <label className="ac-checkbox">
                    <input type="checkbox" checked={syncProfiles} onChange={(e) => setSyncProfiles(e.target.checked)} style={{accentColor: 'var(--ac-accent-cyan)', width: 14, height: 14}} />
                    <span className="ac-checkbox__label">Sync GPU profiles</span>
                  </label>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      <div style={{display: 'flex', justifyContent: 'flex-end'}}>
        <button onClick={handleSave} disabled={saving} className="ac-btn ac-btn--primary" style={{padding: '6px 24px', fontSize: 13}}>
          {saving ? 'Saving...' : 'Save Settings'}
        </button>
      </div>
    </div>
  );
}
