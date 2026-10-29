import { useEffect, useState } from 'react';
import { PanelFrame } from './PanelFrame';
import { overlayService } from '../../services/gpuService';

const COLORS = ['#ff6633', '#ddaa33', '#22c55e', '#3b82f6', '#a855f7', '#ef4444', '#06b6d4', '#eab308'];

export function GameLauncher() {
  const [games, setGames] = useState<string[]>([]);

  useEffect(() => {
    overlayService.getDetectedGames().then(setGames).catch(() => {});
  }, []);

  return (
    <PanelFrame title="Game Launcher" className="ac-panel--flex1">
      <div className="ac-games">
        {games.length === 0 ? (
          <div className="ac-game">
            <span className="ac-game__name" style={{ color: '#555568' }}>No games detected</span>
          </div>
        ) : (
          games.map((game, i) => (
            <div key={game} className="ac-game">
              <div className="ac-game__icon">
                <svg width="48" height="48" viewBox="0 0 48 48">
                  <rect width="48" height="48" fill={`${COLORS[i % COLORS.length]}15`} />
                  <text
                    x="24" y="26"
                    textAnchor="middle"
                    fill={COLORS[i % COLORS.length]}
                    fontSize="14"
                    fontWeight="700"
                    fontFamily="system-ui"
                  >
                    {game.slice(0, 2).toUpperCase()}
                  </text>
                </svg>
              </div>
              <span className="ac-game__name">{game}</span>
            </div>
          ))
        )}
      </div>
    </PanelFrame>
  );
}
