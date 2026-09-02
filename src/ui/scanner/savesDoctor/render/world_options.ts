import { escapeHtml } from '../../rendering';
import { t } from '../../../../utils/i18n';
import { formatDeathPenalty } from '../helpers';
import type { WorldOptionSettings, WorldCustomMeta } from '../../../../api';

export function renderWorldMetaFormHtml(
  customMeta: WorldCustomMeta | null,
  curProfiles: Array<{ id: string; name: string }>,
  curSaving: boolean
): string {
  return `
    <!-- World Custom Nickname, Profile Binding & Notes Card -->
    <div style="background: rgba(0,0,0,0.18); border: 1px solid var(--border); border-radius: 8px; padding: 12px 16px; display: flex; flex-direction: column; gap: 10px; flex-shrink: 0;">
      <div style="display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 8px;">
        <div style="display: flex; align-items: center; gap: 8px;">
          <span style="font-size: 15px;">🏷️</span>
          <span style="font-size: 12.5px; font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.btn_save_meta') || 'World Profile Binding & Notes')}</span>
        </div>
        <button id="btn-save-meta" class="btn-primary" style="padding: 4px 12px; font-size: 11px; display: flex; align-items: center; gap: 5px;" ${curSaving ? 'disabled' : ''}>
          <span>💾</span>
          <span>${curSaving ? 'Saving...' : escapeHtml(t('common.save') || 'Save')}</span>
        </button>
      </div>

      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px;">
        <div style="display: flex; flex-direction: column; gap: 4px;">
          <label style="font-size: 10.5px; color: var(--text-muted); font-weight: 600;">Custom World Nickname</label>
          <input id="meta-nickname-input" type="text" class="input-text" style="font-size: 12px; padding: 6px 10px; background: rgba(0,0,0,0.25); border: 1px solid var(--border); border-radius: 4px; color: var(--text-primary);" placeholder="${escapeHtml(t('scanner.meta_nickname_placeholder') || 'e.g. Main Co-op World')}" value="${escapeHtml(customMeta?.nickname || '')}">
        </div>

        <div style="display: flex; flex-direction: column; gap: 4px;">
          <label style="font-size: 10.5px; color: var(--text-muted); font-weight: 600;">${escapeHtml(t('scanner.meta_bound_profile') || 'Bound Mod Profile:')}</label>
          <select id="meta-profile-select" style="font-size: 12px; padding: 6px 10px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 4px; color: var(--text-primary);">
            <option value="">-- ${escapeHtml(t('scanner.meta_bound_profile_none') || 'None (Use Active Profile)')} --</option>
            ${curProfiles.map(p => `
              <option value="${escapeHtml(p.id)}" ${customMeta?.boundProfileId === p.id ? 'selected' : ''}>${escapeHtml(p.name)}</option>
            `).join('')}
          </select>
        </div>
      </div>

      <div style="display: flex; flex-direction: column; gap: 4px;">
        <label style="font-size: 10.5px; color: var(--text-muted); font-weight: 600;">World Notes & Private Warnings</label>
        <textarea id="meta-notes-input" rows="2" style="font-size: 11.5px; padding: 6px 10px; background: rgba(0,0,0,0.25); border: 1px solid var(--border); border-radius: 4px; color: var(--text-primary); resize: vertical;" placeholder="${escapeHtml(t('scanner.meta_notes_placeholder') || 'Add private notes, mod requirements, or warnings for this world...')}">${escapeHtml(customMeta?.notes || '')}</textarea>
      </div>
    </div>
  `;
}

export function renderWorldOptionsHtml(worldOptions: WorldOptionSettings | null): string {
  if (!worldOptions?.exists) return '';

  return `
    <div style="background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 8px; padding: 14px 16px; display: flex; flex-direction: column; gap: 14px; flex-shrink: 0;">
      <div style="display: flex; justify-content: space-between; align-items: center;">
        <div style="display: flex; align-items: center; gap: 8px;">
          <span style="font-size: 16px;">⚙️</span>
          <span style="font-size: 13px; font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.world_options_title') || 'World Rules & Difficulty (WorldOption.sav)')}</span>
        </div>
        <span class="badge" style="font-size: 10.5px; padding: 3px 10px; background: rgba(56, 189, 248, 0.15); color: #38bdf8; font-weight: 700; border: 1px solid rgba(56, 189, 248, 0.3);">
          ${escapeHtml(worldOptions.difficulty || 'Custom')} Mode
        </span>
      </div>

      <!-- 1. General & Game Pace -->
      <div style="display: flex; flex-direction: column; gap: 6px;">
        <div style="font-size: 11px; font-weight: 700; color: #60a5fa; text-transform: uppercase; letter-spacing: 0.5px; display: flex; align-items: center; gap: 5px;">
          <span>🎯</span> ${escapeHtml(t('scanner.rules_cat_general') || 'General & Game Pace')}
        </div>
        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 6px;">
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_exp_rate') || 'EXP Rate')}:</span>
            <strong style="color: #4af626;">${worldOptions.expRate !== null && worldOptions.expRate !== undefined ? `${worldOptions.expRate}x` : '1.0x'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_day_speed') || 'Day Speed')}:</span>
            <strong style="color: var(--text-primary);">${worldOptions.dayTimeSpeedRate !== null && worldOptions.dayTimeSpeedRate !== undefined ? `${worldOptions.dayTimeSpeedRate}x` : '1.0x'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_night_speed') || 'Night Speed')}:</span>
            <strong style="color: var(--text-primary);">${worldOptions.nightTimeSpeedRate !== null && worldOptions.nightTimeSpeedRate !== undefined ? `${worldOptions.nightTimeSpeedRate}x` : '1.0x'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_work_speed') || 'Work Speed')}:</span>
            <strong style="color: #a3e635;">${worldOptions.workSpeedRate !== null && worldOptions.workSpeedRate !== undefined ? `${worldOptions.workSpeedRate}x` : '1.0x'}</strong>
          </div>
        </div>
      </div>

      <!-- 2. Pals & Capture -->
      <div style="display: flex; flex-direction: column; gap: 6px;">
        <div style="font-size: 11px; font-weight: 700; color: #f59e0b; text-transform: uppercase; letter-spacing: 0.5px; display: flex; align-items: center; gap: 5px;">
          <span>🐾</span> ${escapeHtml(t('scanner.rules_cat_pals') || 'Pals & Capturing')}
        </div>
        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 6px;">
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_pal_capture_rate') || 'Capture Rate')}:</span>
            <strong style="color: #38bdf8;">${worldOptions.palCaptureRate !== null && worldOptions.palCaptureRate !== undefined ? `${worldOptions.palCaptureRate}x` : '1.0x'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_pal_spawn_rate') || 'Pal Spawn Rate')}:</span>
            <strong style="color: var(--text-primary);">${worldOptions.palSpawnNumRate !== null && worldOptions.palSpawnNumRate !== undefined ? `${worldOptions.palSpawnNumRate}x` : '1.0x'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_pal_damage') || 'Pal Damage')}:</span>
            <strong style="color: #fb923c;">${worldOptions.palDamageRate !== null && worldOptions.palDamageRate !== undefined ? `${worldOptions.palDamageRate}x` : '1.0x'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_egg_hatching') || 'Egg Hatching')}:</span>
            <strong style="color: #ffd166;">${worldOptions.palEggHatchingHours !== null && worldOptions.palEggHatchingHours !== undefined ? `${worldOptions.palEggHatchingHours}h` : 'Default'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_pal_hunger') || 'Pal Hunger')}:</span>
            <strong style="color: var(--text-primary);">${worldOptions.palStomachDecreaseRate !== null && worldOptions.palStomachDecreaseRate !== undefined ? `${worldOptions.palStomachDecreaseRate}x` : '1.0x'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_pal_stamina') || 'Pal Stamina')}:</span>
            <strong style="color: var(--text-primary);">${worldOptions.palStaminaDecreaseRate !== null && worldOptions.palStaminaDecreaseRate !== undefined ? `${worldOptions.palStaminaDecreaseRate}x` : '1.0x'}</strong>
          </div>
        </div>
      </div>

      <!-- 3. Player & Survival -->
      <div style="display: flex; flex-direction: column; gap: 6px;">
        <div style="font-size: 11px; font-weight: 700; color: #ef4444; text-transform: uppercase; letter-spacing: 0.5px; display: flex; align-items: center; gap: 5px;">
          <span>⚔️</span> ${escapeHtml(t('scanner.rules_cat_player') || 'Player & Survival')}
        </div>
        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 6px;">
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_player_damage') || 'Player Damage')}:</span>
            <strong style="color: #4af626;">${worldOptions.playerDamageRate !== null && worldOptions.playerDamageRate !== undefined ? `${worldOptions.playerDamageRate}x` : '1.0x'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_death_penalty') || 'Death Penalty')}:</span>
            <strong style="color: ${worldOptions.deathPenalty?.includes('None') ? '#4af626' : '#ffaa00'}; font-size: 10.5px;">${formatDeathPenalty(worldOptions.deathPenalty)}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_player_hunger') || 'Player Hunger')}:</span>
            <strong style="color: var(--text-primary);">${worldOptions.playerStomachDecreaseRate !== null && worldOptions.playerStomachDecreaseRate !== undefined ? `${worldOptions.playerStomachDecreaseRate}x` : '1.0x'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_player_stamina') || 'Player Stamina')}:</span>
            <strong style="color: var(--text-primary);">${worldOptions.playerStaminaDecreaseRate !== null && worldOptions.playerStaminaDecreaseRate !== undefined ? `${worldOptions.playerStaminaDecreaseRate}x` : '1.0x'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_pvp') || 'PvP')}:</span>
            <strong style="color: ${worldOptions.enablePlayerToPlayerDamage ? '#ff5f56' : '#a3e635'};">${worldOptions.enablePlayerToPlayerDamage ? 'Enabled' : 'Disabled'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_friendly_fire') || 'Friendly Fire')}:</span>
            <strong style="color: ${worldOptions.enableFriendlyFire ? '#ff5f56' : '#a3e635'};">${worldOptions.enableFriendlyFire ? 'Enabled' : 'Disabled'}</strong>
          </div>
        </div>
      </div>

      <!-- 4. Base & Guilds -->
      <div style="display: flex; flex-direction: column; gap: 6px;">
        <div style="font-size: 11px; font-weight: 700; color: #10b981; text-transform: uppercase; letter-spacing: 0.5px; display: flex; align-items: center; gap: 5px;">
          <span>🏰</span> ${escapeHtml(t('scanner.rules_cat_bases') || 'Base & Guilds')}
        </div>
        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 6px;">
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_base_workers') || 'Max Base Pals')}:</span>
            <strong style="color: #a3e635;">${worldOptions.baseCampWorkerMaxNum !== null && worldOptions.baseCampWorkerMaxNum !== undefined ? `${worldOptions.baseCampWorkerMaxNum} Pals` : '15'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_max_bases') || 'Max Bases')}:</span>
            <strong style="color: #38bdf8;">${worldOptions.baseCampMaxNum !== null && worldOptions.baseCampMaxNum !== undefined ? `${worldOptions.baseCampMaxNum}` : '3'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_invaders') || 'Base Raids')}:</span>
            <strong style="color: ${worldOptions.enableInvaderEnemy !== false ? '#4af626' : '#ff5f56'};">${worldOptions.enableInvaderEnemy !== false ? 'Enabled' : 'Disabled'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_decay_rate') || 'Structure Decay')}:</span>
            <strong style="color: var(--text-primary);">${worldOptions.buildObjectDeteriorationDamageRate !== null && worldOptions.buildObjectDeteriorationDamageRate !== undefined ? `${worldOptions.buildObjectDeteriorationDamageRate}x` : '1.0x'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_guild_max_players') || 'Guild Max Players')}:</span>
            <strong style="color: var(--text-primary);">${worldOptions.guildPlayerMaxNum !== null && worldOptions.guildPlayerMaxNum !== undefined ? `${worldOptions.guildPlayerMaxNum}` : '20'}</strong>
          </div>
        </div>
      </div>

      <!-- 5. Loot, Drops & World -->
      <div style="display: flex; flex-direction: column; gap: 6px;">
        <div style="font-size: 11px; font-weight: 700; color: #a855f7; text-transform: uppercase; letter-spacing: 0.5px; display: flex; align-items: center; gap: 5px;">
          <span>☄️</span> ${escapeHtml(t('scanner.rules_cat_drops') || 'Loot, Drops & World')}
        </div>
        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 6px;">
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_collection_drop') || 'Drop Multiplier')}:</span>
            <strong style="color: #4af626;">${worldOptions.collectionDropRate !== null && worldOptions.collectionDropRate !== undefined ? `${worldOptions.collectionDropRate}x` : '1.0x'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_enemy_drop') || 'Enemy Drops')}:</span>
            <strong style="color: #38bdf8;">${worldOptions.enemyDropItemRate !== null && worldOptions.enemyDropItemRate !== undefined ? `${worldOptions.enemyDropItemRate}x` : '1.0x'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_resource_respawn') || 'Respawn Speed')}:</span>
            <strong style="color: var(--text-primary);">${worldOptions.collectionObjectRespawnSpeedRate !== null && worldOptions.collectionObjectRespawnSpeedRate !== undefined ? `${worldOptions.collectionObjectRespawnSpeedRate}x` : '1.0x'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_supply_drop_span') || 'Supply Drops')}:</span>
            <strong style="color: #ffd166;">${worldOptions.supplyDropSpan !== null && worldOptions.supplyDropSpan !== undefined ? `${worldOptions.supplyDropSpan} min` : '180 min'}</strong>
          </div>
          <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
            <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_fast_travel') || 'Fast Travel')}:</span>
            <strong style="color: ${worldOptions.enableFastTravel !== false ? '#4af626' : '#ff5f56'};">${worldOptions.enableFastTravel !== false ? 'Enabled' : 'Disabled'}</strong>
          </div>
        </div>
      </div>
    </div>
  `;
}
