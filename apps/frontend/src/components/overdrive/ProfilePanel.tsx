
import { PanelFrame } from './PanelFrame';

/* ================================================================
   ProfilePanel — "My Profile" panel with ROG icon,
   welcome text, Log In and Register buttons.
   ================================================================ */

export function ProfilePanel() {
  return (
    <PanelFrame title="My Profile" className="ac-panel--flex1">
      <div className="ac-profile">
        {/* ROG-style hexagonal icon */}
        <svg className="ac-profile__icon" viewBox="0 0 64 64" fill="none">
          <path
            d="M32 4L56 18V46L32 60L8 46V18L32 4Z"
            fill="#1a1a26"
            stroke="#2a2a38"
            strokeWidth="1.5"
          />
          <path
            d="M32 14L46 22V38L32 46L18 38V22L32 14Z"
            fill="#1e1e2c"
            stroke="#2a2a38"
            strokeWidth="1"
          />
          {/* Inner geometric eye/diamond shape */}
          <path
            d="M32 20L40 26V34L32 40L24 34V26L32 20Z"
            fill="none"
            stroke="#555568"
            strokeWidth="1"
          />
          <circle cx="32" cy="30" r="4" fill="#555568" opacity="0.5" />
        </svg>

        <span className="ac-profile__welcome">Welcome to Overdrive</span>

        <button className="ac-profile__btn">Log In</button>
        <button className="ac-profile__btn">Register Your Product</button>
      </div>
    </PanelFrame>
  );
}
