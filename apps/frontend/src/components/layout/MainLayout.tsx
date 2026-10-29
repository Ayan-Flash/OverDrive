import { TitleBar } from '../overdrive/TitleBar';
import { OverdriveSidebar } from '../overdrive/OverdriveSidebar';
import { OverdriveHeader } from '../overdrive/OverdriveHeader';

/* ================================================================
   MainLayout — Overdrive shell.

   Structure (top→bottom):
     TitleBar        (custom frameless, 32px)
     ┣ OverdriveSidebar (48px icon rail, left)
     ┗ Main area
       ┣ OverdriveHeader (Dashboard title + system specs)
       ┗ Page content  (flex-1, scrolls if needed)
   ================================================================ */

interface MainLayoutProps {
  children: React.ReactNode;
}

export function MainLayout({ children }: MainLayoutProps) {
  return (
    <div
      className="ac-app"
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100vh',
        width: '100vw',
        overflow: 'hidden',
      }}
    >
      <TitleBar />

      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        <OverdriveSidebar />

        <div className="ac-main">
          <OverdriveHeader />
          {children}
        </div>
      </div>
    </div>
  );
}
