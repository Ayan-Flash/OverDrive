import { useState } from 'react';
import { useGpuStore, useProfileStore, useUiStore } from '../../stores';
import { profileService } from '../../services';
import { IconRefresh, IconX } from '../base/Icons';

export function ProfileManager() {
  const [name, setName] = useState('');
  const [saving, setSaving] = useState(false);
  const { controlStates } = useGpuStore();
  const { selectedGpuId } = useUiStore();
  const { profiles, setProfiles } = useProfileStore();

  const control = selectedGpuId ? controlStates.get(selectedGpuId) : null;

  const handleSave = async () => {
    if (!name.trim() || !selectedGpuId || !control) return;
    setSaving(true);
    try {
      await profileService.save(name, selectedGpuId,
        control.core_clock_offset_mhz, control.memory_clock_offset_mhz,
        control.voltage_offset_mv, control.target_fan_speed ?? 50,
        control.power_limit_percent ?? 100);
      setName('');
      const loaded = await profileService.load();
      setProfiles(loaded as typeof profiles);
    } catch (err) {
      console.error('Failed to save profile:', err);
    } finally {
      setSaving(false);
    }
  };

  const handleLoad = async () => {
    try {
      const loaded = await profileService.load();
      setProfiles(loaded as typeof profiles);
    } catch (err) {
      console.error('Failed to load profiles:', err);
    }
  };

  const handleDelete = async (fn: string) => {
    try {
      await profileService.delete(fn);
      await handleLoad();
    } catch (err) {
      console.error('Failed to delete profile:', err);
    }
  };

  return (
    <div className="card flex flex-col gap-4 p-5">
      <div className="section-header">
        <span className="section-title">Profile Manager</span>
      </div>

      <div className="flex gap-2">
        <input className="input flex-1" placeholder="Profile name..."
          value={name} onChange={(e) => setName(e.target.value)} />
        <button onClick={handleSave} disabled={saving || !name.trim()}
          className="btn-primary text-xs disabled:opacity-50">
          {saving ? 'Saving...' : 'Save Current'}
        </button>
      </div>

      <div className="divider" />

      <div className="flex items-center justify-between">
        <span className="text-text-muted text-xs font-medium uppercase tracking-wider">Saved Profiles</span>
        <button onClick={handleLoad} className="btn-ghost p-1" title="Refresh">
          <IconRefresh size={14} />
        </button>
      </div>

      <div className="flex flex-col gap-1.5">
        {profiles.length === 0 && (
          <div className="py-4 text-center text-xs text-text-dim">No saved profiles</div>
        )}
        {profiles.map((profile, i) => (
          <div key={i}
            className="flex items-center justify-between rounded-lg border border-gpu-700 bg-gpu-700/50 px-3 py-2.5 transition-colors hover:bg-gpu-700">
            <div className="flex flex-col">
              <span className="text-sm font-medium text-text-primary">{profile.name}</span>
              <span className="text-text-muted font-mono text-[10px]">
                Core: {profile.core_offset}MHz | Mem: {profile.mem_offset}MHz
              </span>
            </div>
            <button onClick={() => handleDelete(`${profile.name?.replace(/ /g, '_')}_${profile.gpu_id}.json`)}
              className="btn-ghost p-1 text-red-400 hover:text-red-300" title="Delete">
              <IconX size={14} />
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
