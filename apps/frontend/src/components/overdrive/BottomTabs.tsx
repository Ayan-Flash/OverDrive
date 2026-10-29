

/* ================================================================
   BottomTabs — Footer tab bar with 5 metric category tabs.
   ================================================================ */

interface BottomTabsProps {
  activeTab: string;
  onTabChange: (tab: string) => void;
}

const tabs = [
  { id: 'frequency', label: 'Frequency' },
  { id: 'temperature', label: 'Temperature' },
  { id: 'usage', label: 'Usage' },
  { id: 'fan', label: 'Fan' },
  { id: 'voltage', label: 'Voltage' },
];

export function BottomTabs({ activeTab, onTabChange }: BottomTabsProps) {
  return (
    <div className="ac-footer">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          className={`ac-footer__tab ${activeTab === tab.id ? 'ac-footer__tab--active' : ''}`}
          onClick={() => onTabChange(tab.id)}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}
