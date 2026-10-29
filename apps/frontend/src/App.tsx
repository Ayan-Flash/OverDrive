import { useEffect, useMemo } from 'react';
import { useUiStore, useGpuStore } from './stores';
import { MainLayout } from './components/layout';
import {
  DashboardPage,
  ControlPage,
  ProfilesPage,
  AlertsPage,
  SettingsPage,
  RemotePage,
  OverlayPage,
  AutomationPage,
  IntegrationsPage,
  ReportsPage,
  EnterprisePage,
  SyncPage,
  AiPage,
  BackupPage,
  MarketplacePage,
} from './pages';

const pageComponents: Record<string, React.ComponentType> = {
  dashboard: DashboardPage,
  control: ControlPage,
  profiles: ProfilesPage,
  alerts: AlertsPage,
  settings: SettingsPage,
  remote: RemotePage,
  overlay: OverlayPage,
  automation: AutomationPage,
  integrations: IntegrationsPage,
  reports: ReportsPage,
  enterprise: EnterprisePage,
  sync: SyncPage,
  ai: AiPage,
  backup: BackupPage,
  marketplace: MarketplacePage,
};

function SplashScreen() {
  return (
    <div
      style={{
        width: '100vw',
        height: '100vh',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        background: '#0c0c14',
        color: '#8888a0',
        fontFamily: "'Segoe UI', 'Inter', sans-serif",
        gap: 16,
      }}
    >
      <svg width="48" height="48" viewBox="0 0 64 64" fill="none">
        <path d="M32 4L56 18V46L32 60L8 46V18L32 4Z" fill="#161620" stroke="#3a3a4a" strokeWidth="2" />
        <path d="M32 15L45 22.5V37.5L32 45L19 37.5V22.5L32 15Z" fill="#1e1e2c" stroke="#4a4a5e" strokeWidth="1.5" />
        <path d="M32 23L39 27V35L32 39L25 35V27L32 23Z" fill="none" stroke="#7aa8c8" strokeWidth="1.2" />
      </svg>
      <span style={{ fontSize: 13, letterSpacing: '0.5px' }}>OverDrive</span>
    </div>
  );
}

export function App() {
  const { initialized, selectedGpuId, loadSettings, setSelectedGpu, currentPage } = useUiStore();
  const { gpus, fetchGpus } = useGpuStore();

  useEffect(() => {
    loadSettings().catch(() => {});
    fetchGpus().catch(() => {});
  }, [loadSettings, fetchGpus]);

  useEffect(() => {
    if (gpus.length > 0 && !selectedGpuId) {
      setSelectedGpu(gpus[0].id);
    }
  }, [gpus, selectedGpuId, setSelectedGpu]);

  const PageComponent = useMemo(() => pageComponents[currentPage] ?? DashboardPage, [currentPage]);

  if (!initialized) {
    return <SplashScreen />;
  }

  return (
    <MainLayout>
      <PageComponent />
    </MainLayout>
  );
}
