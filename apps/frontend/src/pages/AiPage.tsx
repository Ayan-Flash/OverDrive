import { useEffect, useState } from 'react';
import { useAiStore } from '../stores/aiStore';
import * as aiService from '../services/aiService';
import { useSmartAlertStore } from '../stores/smartAlertStore';
import { IconZap, IconRefresh, IconTrash2, IconCheck, IconSettings, IconSliders, IconBell } from '../components/base/Icons';

function formatDate(ts: number) {
  return new Date(ts * 1000).toLocaleString();
}

function severityBadge(severity: string) {
  switch (severity) {
    case 'Critical': return 'ac-badge--red';
    case 'High': return 'ac-badge--yellow';
    case 'Medium': return 'ac-badge--blue';
    default: return 'ac-badge--blue';
  }
}

function severityBorder(severity: string) {
  switch (severity) {
    case 'Critical': return '#f55';
    case 'High': return '#fbbf24';
    case 'Medium': return '#60a5fa';
    default: return '#60a5fa';
  }
}

function trendIcon(trend: string) {
  switch (trend) {
    case 'Rising': return '\u2191';
    case 'Falling': return '\u2193';
    case 'Stable': return '\u2192';
    case 'Volatile': return '~';
    default: return '?';
  }
}

export function AiPage() {
  const { anomalies, suggestions, tempPrediction, utilPrediction, analyzing, error, fetchAnomalies, fetchSuggestions, clearAnomalies, dismissSuggestion, runAnalysis, predictTemp, predictUtil, clearError } = useAiStore();
  const smartAlertStore = useSmartAlertStore();
  const { fetchAll: fetchSmartAlerts } = smartAlertStore;
  const [selectedGpu, setSelectedGpu] = useState('gpu_0');
  const [activeTab, setActiveTab] = useState<'anomalies' | 'predictions' | 'suggestions' | 'tuning' | 'smart_alerts'>('anomalies');

  const [fanCurveResult, setFanCurveResult] = useState<aiService.FanCurveResult | null>(null);
  const [clockResult, setClockResult] = useState<aiService.ClockTuneResult | null>(null);
  const [powerResult, setPowerResult] = useState<aiService.PowerTuneResult | null>(null);
  const [tuningLoading, setTuningLoading] = useState(false);
  const [maxPower, setMaxPower] = useState(200);

  useEffect(() => {
    fetchAnomalies();
    fetchSuggestions();
    fetchSmartAlerts();
  }, [fetchAnomalies, fetchSuggestions, fetchSmartAlerts]);

  const handlePredict = () => {
    predictTemp(selectedGpu);
    predictUtil(selectedGpu);
  };

  return (
    <div className="ac-page">
      <div className="ac-page-header">
        <div className="ac-page-header__left">
          <div className="ac-page-header__title">AI Insights</div>
          <div className="ac-page-header__desc">Anomaly detection, predictions, and optimization suggestions</div>
        </div>
        <div className="ac-page-header__right">
          <button
            onClick={runAnalysis}
            disabled={analyzing}
            className="ac-btn ac-btn--primary ac-btn--sm"
          >
            {analyzing ? 'Analyzing...' : 'Run Analysis'}
          </button>
        </div>
      </div>

      {error && (
        <div className="ac-banner ac-banner--error">
          {error}
          <button onClick={clearError} className="ac-banner__close">&times;</button>
        </div>
      )}

      <div className="ac-tabs">
        <button onClick={() => setActiveTab('anomalies')} className={`ac-tab ${activeTab === 'anomalies' ? 'ac-tab--active' : ''}`}>
          Anomalies ({anomalies.length})
        </button>
        <button onClick={() => setActiveTab('predictions')} className={`ac-tab ${activeTab === 'predictions' ? 'ac-tab--active' : ''}`}>
          Predictions
        </button>
        <button onClick={() => setActiveTab('suggestions')} className={`ac-tab ${activeTab === 'suggestions' ? 'ac-tab--active' : ''}`}>
          Suggestions ({suggestions.filter(s => !s.applied).length})
        </button>
        <button onClick={() => setActiveTab('tuning')} className={`ac-tab ${activeTab === 'tuning' ? 'ac-tab--active' : ''}`}>
          Auto-Tuning
        </button>
        <button onClick={() => setActiveTab('smart_alerts')} className={`ac-tab ${activeTab === 'smart_alerts' ? 'ac-tab--active' : ''}`}>
          Smart Alerts
        </button>
      </div>

      {activeTab === 'anomalies' && (
        <div style={{display: 'flex', flexDirection: 'column', gap: 16}}>
          <div style={{display: 'flex', justifyContent: 'flex-end'}}>
            <button onClick={() => clearAnomalies()} className="ac-btn ac-btn--ghost ac-btn--sm" style={{color: 'rgba(255,68,68,0.6)'}}>
              <IconTrash2 style={{display: 'inline', width: 14, height: 14, marginRight: 4}} />
              Clear All
            </button>
          </div>
          {anomalies.length === 0 ? (
            <div className="ac-empty">
              <IconZap className="ac-empty__icon" />
              <div className="ac-empty__text">No anomalies detected. Run an analysis to check your GPUs.</div>
            </div>
          ) : (
            <div style={{display: 'flex', flexDirection: 'column', gap: 8}}>
              {[...anomalies].reverse().map((a) => (
                <div key={a.id} className="ac-page-card" style={{borderLeft: `4px solid ${severityBorder(a.severity)}`}}>
                  <div className="ac-page-card__body">
                    <div className="ac-page-header">
                      <div className="ac-page-header__left" style={{minWidth: 0}}>
                        <div style={{display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4}}>
                          <span className={`ac-badge ${severityBadge(a.severity)}`}>
                            {a.severity}
                          </span>
                          <span style={{color: 'var(--ac-text-secondary)', fontSize: 12}}>{a.anomaly_type.replace(/([A-Z])/g, ' $1').trim()}</span>
                          <span style={{color: 'var(--ac-text-muted)', fontSize: 12}}>{a.gpu_id}</span>
                        </div>
                        <p style={{color: 'var(--ac-text-primary)', fontSize: 13}}>{a.message}</p>
                        <p style={{color: 'var(--ac-text-muted)', fontSize: 12, marginTop: 4}}>{formatDate(a.timestamp)}</p>
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {activeTab === 'predictions' && (
        <div style={{display: 'flex', flexDirection: 'column', gap: 16}}>
          <div className="ac-page-card">
            <div className="ac-page-card__body" style={{display: 'flex', flexDirection: 'column', gap: 16}}>
              <div style={{display: 'flex', alignItems: 'center', gap: 12}}>
                <select
                  value={selectedGpu}
                  onChange={(e) => setSelectedGpu(e.target.value)}
                  className="ac-input ac-select"
                >
                  <option value="gpu_0">GPU 0</option>
                  <option value="gpu_1">GPU 1</option>
                </select>
                <button onClick={handlePredict} className="ac-btn ac-btn--primary ac-btn--sm">
                  <IconRefresh style={{display: 'inline', width: 14, height: 14, marginRight: 4}} />
                  Predict
                </button>
              </div>

              <div style={{display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16}}>
                {tempPrediction ? (
                  <div style={{background: 'var(--ac-bg-input)', borderRadius: 8, padding: '12px 16px'}}>
                    <h4 style={{color: 'var(--ac-text-secondary)', fontSize: 12, fontWeight: 500, marginBottom: 8}}>Temperature Forecast</h4>
                    <div style={{display: 'flex', flexDirection: 'column', gap: 6, fontSize: 13}}>
                      <div className="ac-row">
                        <span className="ac-row__label">Current</span>
                        <span className="ac-metric">{tempPrediction.current_value.toFixed(1)}°C</span>
                      </div>
                      <div className="ac-row">
                        <span className="ac-row__label">5 min</span>
                        <span className="ac-metric" style={{color: tempPrediction.trend === 'Rising' ? '#f55' : tempPrediction.trend === 'Falling' ? '#60a5fa' : 'var(--ac-text-primary)'}}>
                          {tempPrediction.predicted_in_5m.toFixed(1)}°C
                        </span>
                      </div>
                      <div className="ac-row">
                        <span className="ac-row__label">15 min</span>
                        <span className="ac-metric" style={{color: tempPrediction.predicted_in_15m > 80 ? '#f55' : 'var(--ac-text-primary)'}}>
                          {tempPrediction.predicted_in_15m.toFixed(1)}°C
                        </span>
                      </div>
                      <div className="ac-row">
                        <span className="ac-row__label">Trend</span>
                        <span style={{color: 'var(--ac-text-primary)', fontSize: 12, fontWeight: 600, fontVariantNumeric: 'tabular-nums'}}>{trendIcon(tempPrediction.trend)} {tempPrediction.trend}</span>
                      </div>
                      <div className="ac-row">
                        <span className="ac-row__label">Confidence</span>
                        <span style={{color: 'var(--ac-text-primary)', fontSize: 12, fontWeight: 600}}>{(tempPrediction.confidence * 100).toFixed(0)}%</span>
                      </div>
                      {tempPrediction.time_to_throttle && (
                        <div className="ac-row">
                          <span style={{color: '#fbbf24', fontSize: 12}}>Time to throttle</span>
                          <span className="ac-metric" style={{color: '#fbbf24'}}>{(tempPrediction.time_to_throttle / 60).toFixed(0)} min</span>
                        </div>
                      )}
                    </div>
                  </div>
                ) : (
                  <div style={{background: 'var(--ac-bg-input)', borderRadius: 8, display: 'flex', alignItems: 'center', justifyContent: 'center', padding: '12px 16px', color: 'var(--ac-text-secondary)', fontSize: 13, minHeight: 180}}>
                    Run a prediction to see temperature forecast
                  </div>
                )}

                {utilPrediction ? (
                  <div style={{background: 'var(--ac-bg-input)', borderRadius: 8, padding: '12px 16px'}}>
                    <h4 style={{color: 'var(--ac-text-secondary)', fontSize: 12, fontWeight: 500, marginBottom: 8}}>Utilization Forecast</h4>
                    <div style={{display: 'flex', flexDirection: 'column', gap: 6, fontSize: 13}}>
                      <div className="ac-row">
                        <span className="ac-row__label">Current</span>
                        <span className="ac-metric">{utilPrediction.current_value.toFixed(1)}%</span>
                      </div>
                      <div className="ac-row">
                        <span className="ac-row__label">5 min</span>
                        <span className="ac-metric">{utilPrediction.predicted_in_5m.toFixed(1)}%</span>
                      </div>
                      <div className="ac-row">
                        <span className="ac-row__label">15 min</span>
                        <span className="ac-metric">{utilPrediction.predicted_in_15m.toFixed(1)}%</span>
                      </div>
                      <div className="ac-row">
                        <span className="ac-row__label">Trend</span>
                        <span style={{color: 'var(--ac-text-primary)', fontSize: 12, fontWeight: 600, fontVariantNumeric: 'tabular-nums'}}>{trendIcon(utilPrediction.trend)} {utilPrediction.trend}</span>
                      </div>
                      <div className="ac-row">
                        <span className="ac-row__label">Confidence</span>
                        <span style={{color: 'var(--ac-text-primary)', fontSize: 12, fontWeight: 600}}>{(utilPrediction.confidence * 100).toFixed(0)}%</span>
                      </div>
                    </div>
                  </div>
                ) : (
                  <div style={{background: 'var(--ac-bg-input)', borderRadius: 8, display: 'flex', alignItems: 'center', justifyContent: 'center', padding: '12px 16px', color: 'var(--ac-text-secondary)', fontSize: 13, minHeight: 180}}>
                    Run a prediction to see utilization forecast
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      )}

      {activeTab === 'suggestions' && (
        <div style={{display: 'flex', flexDirection: 'column', gap: 8}}>
          {suggestions.length === 0 ? (
            <div className="ac-empty">
              <IconSettings className="ac-empty__icon" />
              <div className="ac-empty__text">No optimization suggestions yet. Run an analysis first.</div>
            </div>
          ) : (
            suggestions.map((s) => (
              <div key={s.id} className="ac-page-card" style={{opacity: s.applied ? 0.5 : 1}}>
                <div className="ac-page-card__body">
                  <div className="ac-page-header">
                    <div className="ac-page-header__left" style={{minWidth: 0, gap: 4}}>
                      <div style={{display: 'flex', alignItems: 'center', gap: 8}}>
                        <span style={{background: 'rgba(0,170,220,0.1)', color: 'var(--ac-accent-cyan-bright)', borderRadius: 3, padding: '1px 8px', fontSize: 12, fontWeight: 600}}>
                          {s.category.replace(/([A-Z])/g, ' $1').trim()}
                        </span>
                        {s.applied && (
                          <span className="ac-badge ac-badge--green">Applied</span>
                        )}
                      </div>
                      <p style={{color: 'var(--ac-text-primary)', fontSize: 13, fontWeight: 500}}>{s.title}</p>
                      <p style={{color: 'var(--ac-text-secondary)', fontSize: 12}}>{s.description}</p>
                      <p style={{color: '#34d399', fontSize: 12}}>{s.potential_benefit}</p>
                      <p style={{color: 'var(--ac-text-muted)', fontSize: 12}}>
                        Confidence: {(s.confidence * 100).toFixed(0)}% &middot; {formatDate(s.timestamp)}
                      </p>
                    </div>
                    {!s.applied && (
                      <button onClick={() => dismissSuggestion(s.id)} className="ac-btn ac-btn--ghost ac-btn--sm" style={{color: 'rgba(52,211,153,0.6)'}}>
                        <IconCheck style={{width: 16, height: 16}} />
                      </button>
                    )}
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      )}

      {activeTab === 'tuning' && (
        <div style={{display: 'flex', flexDirection: 'column', gap: 16}}>
          <div className="ac-page-card">
            <div className="ac-page-card__body" style={{display: 'flex', flexDirection: 'column', gap: 16}}>
              <div style={{display: 'flex', alignItems: 'center', gap: 12}}>
                <IconSliders style={{width: 20, height: 20, color: 'var(--ac-accent-cyan-bright)'}} />
                <h3 style={{color: 'var(--ac-text-primary)', fontSize: 13, fontWeight: 600}}>Auto-Tuner</h3>
                <select
                  value={selectedGpu}
                  onChange={(e) => setSelectedGpu(e.target.value)}
                  className="ac-input ac-select"
                  style={{marginLeft: 'auto'}}
                >
                  <option value="gpu_0">GPU 0</option>
                </select>
              </div>

              <div className="ac-grid-3">
                <div style={{background: 'var(--ac-bg-input)', borderRadius: 8, padding: '12px 16px', display: 'flex', flexDirection: 'column'}}>
                  <h4 style={{color: 'var(--ac-text-secondary)', fontSize: 12, fontWeight: 500, marginBottom: 8}}>Fan Curve</h4>
                  <p style={{color: 'var(--ac-text-secondary)', fontSize: 12, marginBottom: 8}}>Analyze your GPU&apos;s thermal behavior and generate an optimal fan curve</p>
                  <button
                    onClick={async () => {
                      setTuningLoading(true);
                      const result = await aiService.tuneFanCurve(selectedGpu);
                      setFanCurveResult(result);
                      setTuningLoading(false);
                    }}
                    disabled={tuningLoading}
                    className="ac-btn ac-btn--primary ac-btn--sm"
                    style={{alignSelf: 'flex-start'}}
                  >
                    Tune Fan Curve
                  </button>
                  {fanCurveResult && (
                    <div style={{display: 'flex', flexDirection: 'column', gap: 4, marginTop: 8, fontSize: 12}}>
                      <p style={{color: 'var(--ac-text-secondary)'}}>Est. max temp: <span style={{color: 'var(--ac-text-primary)', fontWeight: 600}}>{fanCurveResult.estimated_max_temp.toFixed(0)}°C</span></p>
                      <p style={{color: 'var(--ac-text-secondary)'}}>Noise: <span style={{color: 'var(--ac-text-primary)', fontWeight: 600}}>{fanCurveResult.estimated_noise_level}</span></p>
                      <div style={{display: 'flex', flexDirection: 'column', gap: 2, marginTop: 4}}>
                        {fanCurveResult.points.filter(p => p.temperature >= 40).map((p, i) => (
                          <div key={i} style={{display: 'flex', justifyContent: 'space-between', color: 'var(--ac-text-muted)'}}>
                            <span>{p.temperature.toFixed(0)}°C</span>
                            <span style={{color: 'var(--ac-text-primary)', fontWeight: 600}}>{p.fan_speed.toFixed(0)}%</span>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>

                <div style={{background: 'var(--ac-bg-input)', borderRadius: 8, padding: '12px 16px', display: 'flex', flexDirection: 'column'}}>
                  <h4 style={{color: 'var(--ac-text-secondary)', fontSize: 12, fontWeight: 500, marginBottom: 8}}>Clock Offsets</h4>
                  <p style={{color: 'var(--ac-text-secondary)', fontSize: 12, marginBottom: 8}}>Find safe core and memory clock offsets based on your GPU&apos;s thermal headroom</p>
                  <button
                    onClick={async () => {
                      setTuningLoading(true);
                      const result = await aiService.tuneClockOffsets(selectedGpu);
                      setClockResult(result);
                      setTuningLoading(false);
                    }}
                    disabled={tuningLoading}
                    className="ac-btn ac-btn--primary ac-btn--sm"
                    style={{alignSelf: 'flex-start'}}
                  >
                    Tune Clocks
                  </button>
                  {clockResult && (
                    <div style={{display: 'flex', flexDirection: 'column', gap: 4, marginTop: 8, fontSize: 12}}>
                      <div className="ac-row"><span className="ac-row__label">Core offset</span><span style={{color: '#34d399', fontWeight: 600, fontSize: 12}}>+{clockResult.core_offset_mhz} MHz</span></div>
                      <div className="ac-row"><span className="ac-row__label">Mem offset</span><span style={{color: '#34d399', fontWeight: 600, fontSize: 12}}>+{clockResult.memory_offset_mhz} MHz</span></div>
                      <div className="ac-row"><span className="ac-row__label">Stability</span><span style={{color: 'var(--ac-text-primary)', fontWeight: 600, fontSize: 12}}>{clockResult.stability}</span></div>
                      <div className="ac-row"><span className="ac-row__label">Avg temp</span><span style={{color: 'var(--ac-text-primary)', fontWeight: 600, fontSize: 12}}>{clockResult.avg_temperature.toFixed(0)}°C</span></div>
                    </div>
                  )}
                </div>

                <div style={{background: 'var(--ac-bg-input)', borderRadius: 8, padding: '12px 16px', display: 'flex', flexDirection: 'column'}}>
                  <h4 style={{color: 'var(--ac-text-secondary)', fontSize: 12, fontWeight: 500, marginBottom: 8}}>Power Limit</h4>
                  <p style={{color: 'var(--ac-text-secondary)', fontSize: 12, marginBottom: 8}}>Optimize power limit for best efficiency or performance</p>
                  <div style={{display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8}}>
                    <input
                      type="number"
                      value={maxPower}
                      onChange={(e) => setMaxPower(Number(e.target.value))}
                      className="ac-input ac-input--sm"
                      style={{width: 80}}
                    />
                    <span style={{color: 'var(--ac-text-secondary)', fontSize: 12}}>W max</span>
                  </div>
                  <button
                    onClick={async () => {
                      setTuningLoading(true);
                      const result = await aiService.tunePowerLimit(selectedGpu, maxPower);
                      setPowerResult(result);
                      setTuningLoading(false);
                    }}
                    disabled={tuningLoading}
                    className="ac-btn ac-btn--primary ac-btn--sm"
                    style={{alignSelf: 'flex-start'}}
                  >
                    Tune Power
                  </button>
                  {powerResult && (
                    <div style={{display: 'flex', flexDirection: 'column', gap: 4, marginTop: 8, fontSize: 12}}>
                      <div className="ac-row"><span className="ac-row__label">Limit</span><span style={{color: 'var(--ac-text-primary)', fontWeight: 600, fontSize: 12}}>{powerResult.limit_percent.toFixed(0)}%</span></div>
                      <div className="ac-row"><span className="ac-row__label">Perf est.</span><span style={{color: 'var(--ac-text-primary)', fontWeight: 600, fontSize: 12}}>{powerResult.estimated_performance.toFixed(0)}%</span></div>
                      <div className="ac-row"><span className="ac-row__label">Power save</span><span style={{color: '#34d399', fontWeight: 600, fontSize: 12}}>{powerResult.estimated_power_save.toFixed(0)}%</span></div>
                      <div className="ac-row"><span className="ac-row__label">Efficiency</span><span style={{color: 'var(--ac-text-primary)', fontWeight: 600, fontSize: 12}}>x{powerResult.efficiency_score.toFixed(2)}</span></div>
                    </div>
                  )}
                </div>
              </div>

              {tuningLoading && (
                <div style={{display: 'flex', alignItems: 'center', gap: 8, color: 'var(--ac-text-secondary)', fontSize: 13}}>
                  <div className="ac-spinner" />
                  Tuning in progress...
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {activeTab === 'smart_alerts' && (
        <div style={{display: 'flex', flexDirection: 'column', gap: 16}}>
          <div className="ac-page-card">
            <div className="ac-page-card__body" style={{display: 'flex', flexDirection: 'column', gap: 16}}>
              <div className="ac-page-header">
                <div className="ac-page-header__left" style={{display: 'flex', flexDirection: 'row', alignItems: 'center', gap: 12}}>
                  <div style={{display: 'flex', width: 40, height: 40, alignItems: 'center', justifyContent: 'center', borderRadius: 8, background: 'rgba(147,51,234,0.2)'}}>
                    <IconBell style={{width: 20, height: 20, color: '#a78bfa'}} />
                  </div>
                  <div>
                    <h3 style={{color: 'var(--ac-text-primary)', fontSize: 13, fontWeight: 600}}>Smart Alert Engine</h3>
                    <p style={{color: 'var(--ac-text-secondary)', fontSize: 12}}>AI-powered noise reduction and adaptive thresholds</p>
                  </div>
                </div>
                <div className="ac-page-header__right" style={{gap: 4}}>
                  <button onClick={() => smartAlertStore.fetchAll()} className="ac-btn ac-btn--ghost ac-btn--icon" title="Refresh">
                    <IconRefresh style={{width: 16, height: 16}} />
                  </button>
                  <button onClick={() => smartAlertStore.resetBaselines()} className="ac-btn ac-btn--ghost ac-btn--sm" style={{color: 'rgba(255,68,68,0.6)'}}>
                    Reset Baselines
                  </button>
                </div>
              </div>

              <div className="ac-grid-4">
                <div style={{background: 'var(--ac-bg-input)', borderRadius: 8, padding: '8px 12px'}}>
                  <p style={{color: 'var(--ac-text-secondary)', fontSize: 12}}>Context</p>
                  <p style={{color: 'var(--ac-text-primary)', fontSize: 13, fontWeight: 500}}>{smartAlertStore.context?.context ?? 'Unknown'}</p>
                  <p style={{color: 'var(--ac-text-muted)', fontSize: 12}}>{(smartAlertStore.context?.confidence ?? 0) >= 0.5 ? 'High confidence' : 'Low confidence'}</p>
                </div>
                <div style={{background: 'var(--ac-bg-input)', borderRadius: 8, padding: '8px 12px'}}>
                  <p style={{color: 'var(--ac-text-secondary)', fontSize: 12}}>Avg Utilization</p>
                  <p style={{color: 'var(--ac-text-primary)', fontSize: 13, fontWeight: 500}}>{smartAlertStore.context?.avg_utilization?.toFixed(0) ?? '-'}%</p>
                </div>
                <div style={{background: 'var(--ac-bg-input)', borderRadius: 8, padding: '8px 12px'}}>
                  <p style={{color: 'var(--ac-text-secondary)', fontSize: 12}}>Avg Temperature</p>
                  <p style={{color: 'var(--ac-text-primary)', fontSize: 13, fontWeight: 500}}>{smartAlertStore.context?.avg_temperature?.toFixed(0) ?? '-'}°C</p>
                </div>
                <div style={{background: 'var(--ac-bg-input)', borderRadius: 8, padding: '8px 12px'}}>
                  <p style={{color: 'var(--ac-text-secondary)', fontSize: 12}}>Suppressed Alerts</p>
                  <p style={{color: 'var(--ac-text-primary)', fontSize: 13, fontWeight: 500}}>{smartAlertStore.suppressed.reduce((s, a) => s + a.suppress_count, 0)}</p>
                </div>
              </div>
            </div>
          </div>

          <div className="ac-page-card">
            <div className="ac-page-card__header">
              <div className="ac-page-card__title">Learned Baselines</div>
            </div>
            <div className="ac-page-card__body">
              {smartAlertStore.baselines.length === 0 ? (
                <p style={{color: 'var(--ac-text-muted)', fontSize: 12}}>No baselines learned yet. The engine needs more samples.</p>
              ) : (
                <div className="ac-grid-3">
                  {smartAlertStore.baselines.map((b) => (
                    <div key={b.metric} style={{background: 'var(--ac-bg-input)', borderRadius: 8, padding: '8px 12px'}}>
                      <p style={{color: 'var(--ac-accent-cyan-bright)', fontSize: 12, fontWeight: 500, marginBottom: 4, textTransform: 'capitalize'}}>{b.metric.replace(/_/g, ' ')}</p>
                      <div style={{display: 'flex', flexDirection: 'column', gap: 2, fontSize: 12}}>
                        <div className="ac-row"><span className="ac-row__label">Mean</span><span className="ac-metric" style={{fontSize: 12}}>{b.mean.toFixed(1)}</span></div>
                        <div className="ac-row"><span className="ac-row__label">Std Dev</span><span className="ac-metric" style={{fontSize: 12}}>{b.std_dev.toFixed(2)}</span></div>
                        <div className="ac-row"><span className="ac-row__label">Range</span><span className="ac-metric" style={{fontSize: 12}}>{b.min.toFixed(0)} &ndash; {b.max.toFixed(0)}</span></div>
                        <div className="ac-row"><span className="ac-row__label">Samples</span><span className="ac-metric" style={{fontSize: 12}}>{b.sample_count}</span></div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>

          <div className="ac-page-card">
            <div className="ac-page-card__header">
              <div className="ac-page-card__title">Configuration</div>
            </div>
            <div className="ac-page-card__body">
              <div style={{maxWidth: 448, display: 'flex', flexDirection: 'column', gap: 12}}>
                <label className="ac-checkbox">
                  <input
                    type="checkbox"
                    checked={smartAlertStore.config?.enabled ?? true}
                    onChange={(e) => {
                      if (smartAlertStore.config) {
                        smartAlertStore.updateConfig({ ...smartAlertStore.config, enabled: e.target.checked });
                      }
                    }}
                    style={{accentColor: 'var(--ac-accent-cyan)', width: 14, height: 14}}
                  />
                  <span className="ac-checkbox__label">Enable Smart Alerts</span>
                </label>
                <label className="ac-checkbox">
                  <input
                    type="checkbox"
                    checked={smartAlertStore.config?.context_aware ?? true}
                    onChange={(e) => {
                      if (smartAlertStore.config) {
                        smartAlertStore.updateConfig({ ...smartAlertStore.config, context_aware: e.target.checked });
                      }
                    }}
                    style={{accentColor: 'var(--ac-accent-cyan)', width: 14, height: 14}}
                  />
                  <span className="ac-checkbox__label">Context-aware filtering</span>
                </label>
                <label className="ac-checkbox">
                  <input
                    type="checkbox"
                    checked={smartAlertStore.config?.suppress_duplicates ?? true}
                    onChange={(e) => {
                      if (smartAlertStore.config) {
                        smartAlertStore.updateConfig({ ...smartAlertStore.config, suppress_duplicates: e.target.checked });
                      }
                    }}
                    style={{accentColor: 'var(--ac-accent-cyan)', width: 14, height: 14}}
                  />
                  <span className="ac-checkbox__label">Suppress duplicate alerts</span>
                </label>
                <div>
                  <label style={{color: 'var(--ac-text-secondary)', display: 'block', fontSize: 12, fontWeight: 500, marginBottom: 4}}>Adaptive Sensitivity ({smartAlertStore.config?.adaptive_sensitivity.toFixed(1) ?? '1.0'})</label>
                  <input
                    type="range"
                    min="0.1"
                    max="3.0"
                    step="0.1"
                    value={smartAlertStore.config?.adaptive_sensitivity ?? 1.0}
                    onChange={(e) => {
                      if (smartAlertStore.config) {
                        smartAlertStore.updateConfig({ ...smartAlertStore.config, adaptive_sensitivity: parseFloat(e.target.value) });
                      }
                    }}
                    className="ac-slider"
                  />
                  <div style={{display: 'flex', justifyContent: 'space-between', color: 'var(--ac-text-muted)', fontSize: 12}}>
                    <span>Strict</span>
                    <span>Relaxed</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          {smartAlertStore.suppressed.length > 0 && (
            <div className="ac-page-card">
              <div className="ac-page-card__header">
                <div className="ac-page-card__title">Suppressed Alerts</div>
              </div>
              <div className="ac-page-card__body" style={{display: 'flex', flexDirection: 'column', gap: 4}}>
                {smartAlertStore.suppressed.filter(s => s.suppress_count > 0).map((s, i) => (
                  <div key={`${s.rule_id}-${i}`} className="ac-row" style={{background: 'var(--ac-bg-input)', borderRadius: 8, padding: '8px 12px'}}>
                    <div>
                      <p style={{color: 'var(--ac-text-primary)', fontSize: 12}}>{s.message}</p>
                      <p style={{color: 'var(--ac-text-muted)', fontSize: 12}}>Suppressed {s.suppress_count} times</p>
                    </div>
                    <span className="ac-metric" style={{color: '#34d399', fontSize: 12}}>-{s.suppress_count}</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
