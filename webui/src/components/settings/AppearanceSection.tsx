import { createSignal, Show, For } from 'solid-js';
import { Card } from '../core/Card';
import { Toggle } from '../core/Toggle';
import { store } from '../../lib/store';

const accentColors = [
  { name: 'Orange', color: '#FF8E53' },
  { name: 'Emerald', color: '#00D68F' },
  { name: 'Beige', color: '#D4A574' },
  { name: 'Amethyst', color: '#A855F7' },
  { name: 'Fuchsia', color: '#D946EF' },
  { name: 'Lime', color: '#84CC16' },
  { name: 'Indigo', color: '#6366F1' },
  { name: 'Coral', color: '#FF6B6B' },
];

export function AppearanceSection() {
  const [glassOpen, setGlassOpen] = createSignal(false);
  const selectedAccent = () => store.settings.accentColor;
  const isThemeActive = (themeName: string) => store.settings.theme === themeName;

  const handleThemeChange = (newTheme: 'dark' | 'light' | 'auto' | 'amoled') => {
    store.updateSettings({ theme: newTheme });
  };

  return (
    <Card>
      <h3 class="settings__section-title">
        <svg class="settings__section-icon" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 3c-4.97 0-9 4.03-9 9s4.03 9 9 9c.83 0 1.5-.67 1.5-1.5 0-.39-.15-.74-.39-1.01-.23-.26-.38-.61-.38-.99 0-.83.67-1.5 1.5-1.5H16c2.76 0 5-2.24 5-5 0-4.42-4.03-8-9-8zm-5.5 9c-.83 0-1.5-.67-1.5-1.5S5.67 9 6.5 9 8 9.67 8 10.5 7.33 12 6.5 12zm3-4C8.67 8 8 7.33 8 6.5S8.67 5 9.5 5s1.5.67 1.5 1.5S10.33 8 9.5 8zm5 0c-.83 0-1.5-.67-1.5-1.5S13.67 5 14.5 5s1.5.67 1.5 1.5S15.33 8 14.5 8zm3 4c-.83 0-1.5-.67-1.5-1.5S16.67 9 17.5 9s1.5.67 1.5 1.5-.67 1.5-1.5 1.5z"/>
        </svg>
        Appearance
      </h3>

      <div class="settings__group">
        <div class="settings__label">Theme</div>
        <div class="settings__themes">
          <button
            class={`settings__theme ${isThemeActive('dark') ? 'settings__theme--active' : ''}`}
            onClick={() => handleThemeChange('dark')}
          >
            <div class="settings__theme-icon">
              <svg width="24" height="24" viewBox="0 0 24 24" fill={isThemeActive('dark') ? 'var(--text-accent)' : 'var(--text-secondary)'}>
                <path d="M12.43 2.3c-2.38-.59-4.68-.27-6.63.64-.35.16-.41.64-.1.86C8.3 5.6 10 8.6 10 12c0 3.4-1.7 6.4-4.3 8.2-.32.22-.26.7.09.86 1.28.6 2.71.94 4.21.94 6.05 0 10.85-5.38 9.87-11.6-.61-3.92-3.59-7.16-7.44-8.1z"/>
              </svg>
            </div>
            <div class={`settings__theme-label ${isThemeActive('dark') ? 'settings__theme-label--active' : ''}`}>
              Dark
            </div>
          </button>

          <button
            class={`settings__theme ${isThemeActive('light') ? 'settings__theme--active' : ''}`}
            onClick={() => handleThemeChange('light')}
          >
            <div class="settings__theme-icon">
              <svg width="24" height="24" viewBox="0 0 24 24" fill={isThemeActive('light') ? 'var(--text-accent)' : 'var(--text-secondary)'}>
                <path d="M6.76 4.84l-1.8-1.79-1.41 1.41 1.79 1.79 1.42-1.41zM4 10.5H1v2h3v-2zm9-9.95h-2V3.5h2V.55zm7.45 3.91l-1.41-1.41-1.79 1.79 1.41 1.41 1.79-1.79zm-3.21 13.7l1.79 1.8 1.41-1.41-1.8-1.79-1.4 1.4zM20 10.5v2h3v-2h-3zm-8-5c-3.31 0-6 2.69-6 6s2.69 6 6 6 6-2.69 6-6-2.69-6-6-6zm-1 16.95h2V19.5h-2v2.95zm-7.45-3.91l1.41 1.41 1.79-1.8-1.41-1.41-1.79 1.8z"/>
              </svg>
            </div>
            <div class={`settings__theme-label ${isThemeActive('light') ? 'settings__theme-label--active' : ''}`}>
              Light
            </div>
          </button>

          <button
            class={`settings__theme ${isThemeActive('auto') ? 'settings__theme--active' : ''}`}
            onClick={() => handleThemeChange('auto')}
          >
            <div class="settings__theme-icon">
              <svg width="24" height="24" viewBox="0 0 24 24" fill={isThemeActive('auto') ? 'var(--text-accent)' : 'var(--text-secondary)'}>
                <path d="M12 4V2A10 10 0 0 0 2 12h2a8 8 0 0 1 8-8zm0 16a8 8 0 0 1-8-8H2a10 10 0 0 0 10 10v-2zm8-8a8 8 0 0 1-8 8v2a10 10 0 0 0 10-10h-2zm-8-8a8 8 0 0 1 8 8h2A10 10 0 0 0 12 2v2z"/>
              </svg>
            </div>
            <div class={`settings__theme-label ${isThemeActive('auto') ? 'settings__theme-label--active' : ''}`}>
              Auto
            </div>
          </button>

          <button
            class={`settings__theme ${isThemeActive('amoled') ? 'settings__theme--active' : ''}`}
            onClick={() => handleThemeChange('amoled')}
          >
            <div class="settings__theme-icon">
              <svg width="24" height="24" viewBox="0 0 24 24" fill={isThemeActive('amoled') ? 'var(--text-accent)' : 'var(--text-secondary)'}>
                <circle cx="12" cy="12" r="10"/>
              </svg>
            </div>
            <div class={`settings__theme-label ${isThemeActive('amoled') ? 'settings__theme-label--active' : ''}`}>
              AMOLED
            </div>
          </button>
        </div>
      </div>

      <div class={`settings__group ${store.settings.autoAccentColor ? 'settings__group--disabled' : ''}`}>
        <div class="settings__label">Accent Color</div>
        <div class="settings__colors">
          <For each={accentColors}>
            {(accent) => (
              <button
                class={`settings__color ${selectedAccent() === accent.color ? 'settings__color--active' : ''} ${store.settings.autoAccentColor ? 'settings__color--disabled' : ''}`}
                onClick={() => {
                  if (!store.settings.autoAccentColor) {
                    store.updateSettings({ accentColor: accent.color });
                  }
                }}
                disabled={store.settings.autoAccentColor}
                style={{
                  background: accent.color,
                  "box-shadow": selectedAccent() === accent.color ? `0 0 0 3px ${accent.color}40` : 'none'
                }}
              />
            )}
          </For>
        </div>
      </div>

      <div class="settings__item">
        <div class="settings__item-content">
          <div class="settings__item-label">Random Accent</div>
          <div class="settings__item-desc">Change accent color each session</div>
        </div>
        <Toggle
          checked={store.settings.autoAccentColor}
          onChange={async (checked) => {
            store.updateSettings({ autoAccentColor: checked });
            if (checked) {
              await store.fetchSystemColor();
            }
          }}
        />
      </div>

      <div class="settings__glass-row" onClick={() => setGlassOpen(!glassOpen())}>
        <div class="settings__item-content">
          <div class="settings__item-label">Glass Intensity</div>
          <div class="settings__item-desc">Background frosted glass effect</div>
        </div>
        <div class="settings__glass-badge">
          <span>{Math.round(store.bgOpacity() * 100)}%</span>
          <svg class={`settings__glass-chevron${glassOpen() ? ' settings__glass-chevron--open' : ''}`} width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
            <path d="M7 10l5 5 5-5z"/>
          </svg>
        </div>
      </div>
      <Show when={glassOpen()}>
        <div class="settings__glass-slider">
          <input
            type="range"
            class="settings__slider"
            min="0"
            max="100"
            value={Math.round(store.bgOpacity() * 100)}
            onInput={(e) => store.setBgOpacity(parseInt(e.currentTarget.value) / 100)}
          />
        </div>
      </Show>

      <div class="settings__item">
        <div class="settings__item-content">
          <div class="settings__item-label">Fix Bottom Nav</div>
          <div class="settings__item-desc">Pin navigation to bottom of screen</div>
        </div>
        <Toggle
          checked={store.settings.fixedNav}
          onChange={(checked) => store.updateSettings({ fixedNav: checked })}
        />
      </div>
    </Card>
  );
}
