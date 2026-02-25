import { createSignal, Show } from 'solid-js';
import { Card } from '../core/Card';
import { Toggle } from '../core/Toggle';
import { BottomSheet } from '../ui/BottomSheet';
import { ChipSelect } from '../ui/ChipSelect';
import { store } from '../../lib/store';
import type { StorageMode } from '../../lib/types';

export function MountEngineSection() {
  const [customOverlaySource, setCustomOverlaySource] = createSignal('');
  const [customMountSource, setCustomMountSource] = createSignal('');
  const [showOverlaySheet, setShowOverlaySheet] = createSignal(false);
  const [showStagingSheet, setShowStagingSheet] = createSignal(false);

  const caps = () => store.capabilities?.() || null;

  return (
    <Card>
      <h3 class="settings__section-title">
        <svg class="settings__section-icon" viewBox="0 0 24 24" fill="currentColor">
          <path d="M20 6h-8l-2-2H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2zm-6 10H6v-2h8v2zm4-4H6v-2h12v2z"/>
        </svg>
        Mount Engine
      </h3>

      <div class="settings__group">
        <div class="settings__item-label">Mount Strategy</div>
        <div class="settings__item-desc" style={{ "margin-bottom": "12px" }}>
          {caps()?.vfs_driver
            ? 'VFS driver detected — auto-selects VFS when kernel driver is present'
            : 'No VFS driver — controls overlay vs magic mount preference'}
        </div>
        <div class="settings__strategies">
          <button
            class={`settings__strategy${store.effectiveStrategy() === 'Vfs' ? ' settings__strategy--active' : ''}${!caps()?.vfs_driver ? ' settings__strategy--disabled' : ''}`}
            onClick={() => store.setMountStrategy('Vfs')}
            disabled={!caps()?.vfs_driver}
            title={!caps()?.vfs_driver ? 'VFS kernel driver not available' : 'Auto: VFS when driver present, overlay/magic fallback'}
          >
            <div style={{ "margin-bottom": "4px" }}>
              <svg width="24" height="24" viewBox="0 0 24 24" fill={store.effectiveStrategy() === 'Vfs' ? 'var(--text-accent)' : 'var(--text-secondary)'}>
                <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"/>
              </svg>
            </div>
            <div class={`settings__strategy-label${store.effectiveStrategy() === 'Vfs' ? ' settings__strategy-label--active' : ''}`}>
              VFS
            </div>
            <div class="settings__strategy-hint">Auto / Kernel</div>
          </button>

          <button
            class={`settings__strategy${store.effectiveStrategy() === 'Overlay' ? ' settings__strategy--active' : ''}${!caps()?.overlay_supported ? ' settings__strategy--disabled' : ''}`}
            onClick={() => store.setMountStrategy('Overlay')}
            disabled={!caps()?.overlay_supported}
            title={!caps()?.overlay_supported ? 'OverlayFS not supported on this kernel' : 'Prefer OverlayFS stacked filesystem'}
          >
            <div style={{ "margin-bottom": "4px" }}>
              <svg width="24" height="24" viewBox="0 0 24 24" fill={store.effectiveStrategy() === 'Overlay' ? 'var(--text-accent)' : 'var(--text-secondary)'}>
                <path d="M4 6H2v14c0 1.1.9 2 2 2h14v-2H4V6zm16-4H8c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zm0 14H8V4h12v12z"/>
              </svg>
            </div>
            <div class={`settings__strategy-label${store.effectiveStrategy() === 'Overlay' ? ' settings__strategy-label--active' : ''}`}>
              Overlay
            </div>
            <div class="settings__strategy-hint">OverlayFS</div>
          </button>

          <button
            class={`settings__strategy${store.effectiveStrategy() === 'MagicMount' ? ' settings__strategy--active' : ''}`}
            onClick={() => store.setMountStrategy('MagicMount')}
            title="Bind mounts (Magisk-style) — always available"
          >
            <div style={{ "margin-bottom": "4px" }}>
              <svg width="24" height="24" viewBox="0 0 24 24" fill={store.effectiveStrategy() === 'MagicMount' ? 'var(--text-accent)' : 'var(--text-secondary)'}>
                <path d="M17 1.01L7 1c-1.1 0-2 .9-2 2v18c0 1.1.9 2 2 2h10c1.1 0 2-.9 2-2V3c0-1.1-.9-1.99-2-1.99zM17 19H7V5h10v14z"/>
              </svg>
            </div>
            <div class={`settings__strategy-label${store.effectiveStrategy() === 'MagicMount' ? ' settings__strategy-label--active' : ''}`}>
              Magic
            </div>
            <div class="settings__strategy-hint">Bind Mount</div>
          </button>
        </div>
        <div class="settings__item-desc" style={{ "margin-top": "8px", "font-style": "italic" }}>
          Switching mode requires reboot
        </div>
      </div>

      <Show when={store.effectiveStrategy() !== 'Vfs'}>
        <Show when={store.effectiveStrategy() === 'Overlay'}>
          <div class="settings__group" style={{ "margin-top": "16px" }}>
            <div class="settings__item-label">Storage Backend</div>
            <div class="settings__item-desc" style={{ "margin-bottom": "10px" }}>
              Filesystem for staging module content
              {caps()?.tmpfs_xattr ? '' : ' (tmpfs lacks xattr — overlay whiteouts unavailable)'}
            </div>
            <ChipSelect
              value={store.settings.mount.storage_mode}
              onChange={(v) => store.setMountStorageMode(v as StorageMode)}
              options={[
                { value: 'auto', label: 'Auto' },
                { value: 'erofs', label: 'EROFS', disabled: !caps()?.erofs_supported },
                { value: 'tmpfs', label: 'tmpfs', disabled: !caps()?.tmpfs_xattr },
                { value: 'ext4', label: 'ext4' },
              ]}
            />
            <Show when={
              store.resolvedStorageMode() &&
              store.settings.mount.storage_mode !== 'auto' &&
              store.resolvedStorageMode() !== store.settings.mount.storage_mode
            }>
              <div class="settings__item-desc" style={{ color: 'var(--warning)', "margin-top": "8px" }}>
                {store.settings.mount.storage_mode} unavailable — resolved to {store.resolvedStorageMode()}
              </div>
            </Show>
          </div>
        </Show>

        <div class="settings__item">
          <div class="settings__item-content">
            <div class="settings__item-label">Random Mount Paths</div>
            <div class="settings__item-desc">Randomize staging directory names at boot</div>
          </div>
          <Toggle
            checked={store.settings.mount.random_mount_paths}
            onChange={(v) => store.setMountToggle('random_mount_paths', v)}
          />
        </div>

        <Show when={store.effectiveStrategy() !== 'MagicMount'}>
          <div class="settings__item">
            <div class="settings__item-content">
              <div class="settings__item-label">Overlay Mount Source</div>
              <div class="settings__item-desc">
                Source device for overlay mounts
              </div>
            </div>
            <button class="settings__select-trigger" onClick={() => setShowOverlaySheet(true)}>
              <span>{['auto', 'KSU', 'magisk', 'overlay'].includes(store.settings.mount.overlay_source) ? store.settings.mount.overlay_source : 'Custom'}</span>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M7 10l5 5 5-5z"/></svg>
            </button>
          </div>
          <BottomSheet
            open={showOverlaySheet()}
            onClose={() => setShowOverlaySheet(false)}
            title="Overlay Mount Source"
            value={['auto', 'KSU', 'magisk', 'overlay'].includes(store.settings.mount.overlay_source) ? store.settings.mount.overlay_source : 'custom'}
            onChange={(val) => {
              if (val !== 'custom') {
                store.setOverlaySource(val);
              } else {
                setCustomOverlaySource('');
              }
            }}
            options={[
              { value: 'auto', label: 'Auto', description: 'Resolves per root manager (KSU/Magisk)' },
              { value: 'KSU', label: 'KSU', description: 'KernelSU device label' },
              { value: 'magisk', label: 'magisk', description: 'Magisk device label' },
              { value: 'overlay', label: 'overlay', description: 'Generic overlay device' },
              { value: 'custom', label: 'Custom', description: 'Enter a custom device label' },
            ]}
            customInput={{
              placeholder: 'e.g. my_overlay',
              value: customOverlaySource(),
              onInput: setCustomOverlaySource,
              onConfirm: (v) => store.setOverlaySource(v),
            }}
          />
        </Show>

        <div class="settings__item">
          <div class="settings__item-content">
            <div class="settings__item-label">Staging Mount Source</div>
            <div class="settings__item-desc">
              Source device for staging mounts
            </div>
          </div>
          <button class="settings__select-trigger" onClick={() => setShowStagingSheet(true)}>
            <span>{['auto', 'tmpfs', 'none', 'shmem', 'shm'].includes(store.settings.mount.mount_source) ? store.settings.mount.mount_source : 'Custom'}</span>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M7 10l5 5 5-5z"/></svg>
          </button>
        </div>
        <BottomSheet
          open={showStagingSheet()}
          onClose={() => setShowStagingSheet(false)}
          title="Staging Mount Source"
          value={['auto', 'tmpfs', 'none', 'shmem', 'shm'].includes(store.settings.mount.mount_source) ? store.settings.mount.mount_source : 'custom'}
          onChange={(val) => {
            if (val !== 'custom') {
              store.setMountSource(val);
            } else {
              setCustomMountSource('');
            }
          }}
          options={[
            { value: 'auto', label: 'Auto', description: 'Random selection per boot' },
            { value: 'tmpfs', label: 'tmpfs', description: 'RAM-backed temporary filesystem' },
            { value: 'none', label: 'none', description: 'VFS tmpfs mount' },
            { value: 'shmem', label: 'shmem', description: 'Shared memory mount' },
            { value: 'shm', label: 'shm', description: 'POSIX shared memory' },
            { value: 'custom', label: 'Custom', description: 'Enter a custom device source' },
          ]}
          customInput={{
            placeholder: 'e.g. my_source',
            value: customMountSource(),
            onInput: setCustomMountSource,
            onConfirm: (v) => store.setMountSource(v),
          }}
        />
      </Show>
    </Card>
  );
}
