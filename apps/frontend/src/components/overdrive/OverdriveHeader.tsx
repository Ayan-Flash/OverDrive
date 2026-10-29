import { useEffect, useState } from 'react';
import { useGpuStore, useUiStore } from '../../stores';
import { cpuService, type CpuInfo } from '../../services/cpuService';

type PageId = Parameters<ReturnType<typeof useUiStore.getState>['navigateTo']>[0];

const pageTitles: Record<PageId, string> = {
  dashboard: 'Overview',
  control: 'Control',
  profiles: 'Profiles',
  alerts: 'Alerts',
  remote: 'Remote Monitoring',
  overlay: 'Gaming Overlay',
  automation: 'Automation',
  integrations: 'Integrations',
  reports: 'Performance Lab',
  enterprise: 'Enterprise',
  sync: 'Cloud Sync',
  ai: 'AI Insights',
  backup: 'Backup & Restore',
  marketplace: 'Profile Marketplace',
  monitoring: 'Monitor',
  settings: 'Settings',
};

export function OverdriveHeader() {
  const currentPage = useUiStore((state) => state.currentPage);
  const selectedGpuId = useUiStore((state) => state.selectedGpuId);
  const gpus = useGpuStore((state) => state.gpus);
  const [cpuInfo, setCpuInfo] = useState<CpuInfo | null>(null);

  useEffect(() => {
    let mounted = true;
    cpuService.getInfo().then((info) => {
      if (mounted) setCpuInfo(info);
    }).catch(() => {
      if (mounted) setCpuInfo(null);
    });
    return () => {
      mounted = false;
    };
  }, []);

  const selectedGpu = gpus.find((gpu) => gpu.id === selectedGpuId);
  const title = pageTitles[currentPage];

  return (
    <header className="ac-header">
      <div className="ac-header__heading">
        <span className="ac-header__eyebrow">OVERDRIVE</span>
        <h1 className="ac-header__title">{title}</h1>
      </div>
      <div className="ac-header__specs" aria-label="Detected hardware">
        {cpuInfo?.model && <span className="ac-header__hardware"><span className="ac-header__specs-dot" />{cpuInfo.model}</span>}
        {selectedGpu?.name && <span className="ac-header__hardware"><span className="ac-header__specs-dot ac-header__specs-dot--teal" />{selectedGpu.name}</span>}
      </div>
    </header>
  );
}

