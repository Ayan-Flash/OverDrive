

/* ================================================================
   CpuCoreCards — 2×2 grid of CPU core frequency cards.
   Each card has a left accent bar, core label, and frequency value.
   ================================================================ */

interface CoreData {
  coreIndex: number;
  frequency: number; // MHz
  usage: number; // percent, per logical processor
}

interface CpuCoreCardsProps {
  cores: CoreData[];
}

export function CpuCoreCards({ cores }: CpuCoreCardsProps) {
  return (
    <div className="ac-core-cards">
      {cores.map((core, i) => {
        const isRed = i % 2 === 1;
        const colorClass = isRed ? 'red' : 'blue';

        return (
          <div key={core.coreIndex} className={`ac-core-card ac-core-card--${colorClass}`}>
            <div className={`ac-core-card__icon ac-core-card__icon--${colorClass}`}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke={isRed ? '#cc3333' : '#2277cc'} strokeWidth="2">
                <rect x="4" y="4" width="16" height="16" rx="2" />
                <rect x="9" y="9" width="6" height="6" />
                <line x1="9" y1="1" x2="9" y2="4" />
                <line x1="15" y1="1" x2="15" y2="4" />
                <line x1="9" y1="20" x2="9" y2="23" />
                <line x1="15" y1="20" x2="15" y2="23" />
                <line x1="20" y1="9" x2="23" y2="9" />
                <line x1="20" y1="14" x2="23" y2="14" />
                <line x1="1" y1="9" x2="4" y2="9" />
                <line x1="1" y1="14" x2="4" y2="14" />
              </svg>
            </div>
            <div className="ac-core-card__info">
              <div className="ac-core-card__label">Thread #{core.coreIndex}</div>
              <div className="ac-core-card__value">
                {core.frequency}
                <span className="ac-core-card__unit">MHz</span>
              </div>
              <div className="ac-core-card__label" style={{ marginTop: 2, color: isRed ? '#ff5555' : '#3399ff', fontWeight: 600 }}>
                {Math.round(core.usage)}% usage
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}
