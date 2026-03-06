// ============================================================
// FILE:        PlanWizard.tsx
// MODULE:      Layer 4 — UI Shell > Plan Creation Wizard
// TASK:        T-036
// PLATFORM:    windows, macos, linux (Tauri)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 2, Plan creation wizard UI
// DEPENDENCIES: React 18, TypeScript, Tauri invoke API
// TEST COVERAGE: U-02 (create plan → appears in plan list)
// KNOWN LIMITATIONS: Tauri window may lag on first paint (~300ms).
//                    Complex schedule UI requires time picker component.
// ============================================================

import React, { useState, useCallback } from "react";

// ============ Types ============

/** Plan type matches policy_schema_v1.json */
interface Plan {
  id: string;
  name: string;
  enabled: boolean;
  schedules: Schedule[];
  app_rules: AppRule[];
  url_rules: UrlRule[];
  quotas: Quota[];
  forced_mode: ForcedModeConfig | null;
}

interface Schedule {
  days: string[];            // "mon"|"tue"|"wed"|"thu"|"fri"|"sat"|"sun"
  start_time: string;         // "HH:MM" 24h
  end_time: string;           // "HH:MM" 24h
  timezone: string;           // IANA timezone
}

interface AppRule {
  process_name?: string;
  path_prefix?: string;
  path_exact?: string;
  bundle_id?: string;
  action: "block" | "allow";
}

interface UrlRule {
  domain: string;
  path_pattern?: string;
  action: "block" | "allow";
}

interface Quota {
  target: string;
  target_type: "app" | "domain";
  daily_seconds?: number;
  weekly_seconds?: number;
  session_seconds?: number;
}

interface ForcedModeConfig {
  enabled: boolean;
  duration_minutes: number;
  password_protected: boolean;
}

/** Wizard step */
type WizardStep = "basics" | "schedule" | "apps" | "urls" | "quotas" | "forced" | "review";

const WIZARD_STEPS: WizardStep[] = [
  "basics", "schedule", "apps", "urls", "quotas", "forced", "review"
];

const STEP_LABELS: Record<WizardStep, string> = {
  basics: "Plan Basics",
  schedule: "Schedule",
  apps: "App Rules",
  urls: "URL Rules",
  quotas: "Quotas",
  forced: "Forced Mode",
  review: "Review & Save",
};

const DAYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

// ============ Component ============

/**
 * PlanWizard — multi-step wizard for creating or editing a FocusMe plan.
 *
 * Steps:
 * 1. Basics: plan name, enabled toggle
 * 2. Schedule: day/time schedule(s), timezone
 * 3. Apps: app blocking rules (process name, path, bundle ID)
 * 4. URLs: domain blocking rules
 * 5. Quotas: daily/weekly/session time limits
 * 6. Forced Mode: lockdown configuration
 * 7. Review: summary + save
 */
export const PlanWizard: React.FC<{
  existingPlan?: Plan;
  onSave: (plan: Plan) => void;
  onCancel: () => void;
}> = ({ existingPlan, onSave, onCancel }) => {
  const [currentStep, setCurrentStep] = useState<WizardStep>("basics");
  const [plan, setPlan] = useState<Plan>(
    existingPlan ?? {
      id: crypto.randomUUID(),
      name: "",
      enabled: true,
      schedules: [],
      app_rules: [],
      url_rules: [],
      quotas: [],
      forced_mode: null,
    }
  );

  const currentStepIndex = WIZARD_STEPS.indexOf(currentStep);
  const isFirstStep = currentStepIndex === 0;
  const isLastStep = currentStepIndex === WIZARD_STEPS.length - 1;

  const goNext = useCallback(() => {
    if (!isLastStep) setCurrentStep(WIZARD_STEPS[currentStepIndex + 1]);
  }, [currentStepIndex, isLastStep]);

  const goBack = useCallback(() => {
    if (!isFirstStep) setCurrentStep(WIZARD_STEPS[currentStepIndex - 1]);
  }, [currentStepIndex, isFirstStep]);

  const handleSave = useCallback(() => {
    // TODO: Validate plan against policy_schema_v1.json
    // TODO: Send to daemon via Tauri invoke("create_plan", { plan })
    onSave(plan);
  }, [plan, onSave]);

  return (
    <div className="plan-wizard">
      {/* Step Progress Bar */}
      <nav className="wizard-steps" aria-label="Plan wizard steps">
        {WIZARD_STEPS.map((step, i) => (
          <button
            key={step}
            className={`wizard-step ${step === currentStep ? "active" : ""} ${i < currentStepIndex ? "completed" : ""}`}
            onClick={() => setCurrentStep(step)}
            aria-current={step === currentStep ? "step" : undefined}
          >
            <span className="step-number">{i + 1}</span>
            <span className="step-label">{STEP_LABELS[step]}</span>
          </button>
        ))}
      </nav>

      {/* Step Content */}
      <div className="wizard-content">
        {currentStep === "basics" && (
          <StepBasics plan={plan} onChange={setPlan} />
        )}
        {currentStep === "schedule" && (
          <StepSchedule plan={plan} onChange={setPlan} />
        )}
        {currentStep === "apps" && (
          <StepApps plan={plan} onChange={setPlan} />
        )}
        {currentStep === "urls" && (
          <StepUrls plan={plan} onChange={setPlan} />
        )}
        {currentStep === "quotas" && (
          <StepQuotas plan={plan} onChange={setPlan} />
        )}
        {currentStep === "forced" && (
          <StepForcedMode plan={plan} onChange={setPlan} />
        )}
        {currentStep === "review" && (
          <StepReview plan={plan} />
        )}
      </div>

      {/* Navigation Buttons */}
      <div className="wizard-actions">
        <button onClick={onCancel} className="btn-secondary">
          Cancel
        </button>
        <div className="wizard-nav">
          {!isFirstStep && (
            <button onClick={goBack} className="btn-secondary">
              Back
            </button>
          )}
          {isLastStep ? (
            <button onClick={handleSave} className="btn-primary">
              {existingPlan ? "Update Plan" : "Create Plan"}
            </button>
          ) : (
            <button onClick={goNext} className="btn-primary">
              Next
            </button>
          )}
        </div>
      </div>
    </div>
  );
};

// ============ Step Components ============

const StepBasics: React.FC<{ plan: Plan; onChange: (p: Plan) => void }> = ({
  plan,
  onChange,
}) => (
  <section>
    <h2>Plan Basics</h2>
    <div className="form-group">
      <label htmlFor="plan-name">Plan Name</label>
      <input
        id="plan-name"
        type="text"
        value={plan.name}
        onChange={(e) => onChange({ ...plan, name: e.target.value })}
        placeholder="e.g., Work Focus, Study Time"
        maxLength={100}
        required
      />
    </div>
    <div className="form-group">
      <label>
        <input
          type="checkbox"
          checked={plan.enabled}
          onChange={(e) => onChange({ ...plan, enabled: e.target.checked })}
        />
        Enable plan immediately
      </label>
    </div>
  </section>
);

const StepSchedule: React.FC<{ plan: Plan; onChange: (p: Plan) => void }> = ({
  plan,
  onChange,
}) => {
  const addSchedule = () => {
    onChange({
      ...plan,
      schedules: [
        ...plan.schedules,
        { days: ["mon", "tue", "wed", "thu", "fri"], start_time: "09:00", end_time: "17:00", timezone: Intl.DateTimeFormat().resolvedOptions().timeZone },
      ],
    });
  };

  return (
    <section>
      <h2>Schedule</h2>
      <p>Define when this plan should be active.</p>

      {plan.schedules.map((schedule, i) => (
        <div key={i} className="schedule-entry">
          <div className="form-group">
            <label>Days</label>
            <div className="day-checkboxes">
              {DAYS.map((day) => (
                <label key={day}>
                  <input
                    type="checkbox"
                    checked={schedule.days.includes(day)}
                    onChange={(e) => {
                      const newDays = e.target.checked
                        ? [...schedule.days, day]
                        : schedule.days.filter((d) => d !== day);
                      const newSchedules = [...plan.schedules];
                      newSchedules[i] = { ...schedule, days: newDays };
                      onChange({ ...plan, schedules: newSchedules });
                    }}
                  />
                  {day.charAt(0).toUpperCase() + day.slice(1)}
                </label>
              ))}
            </div>
          </div>
          <div className="form-row">
            <div className="form-group">
              <label>Start Time</label>
              <input
                type="time"
                value={schedule.start_time}
                onChange={(e) => {
                  const newSchedules = [...plan.schedules];
                  newSchedules[i] = { ...schedule, start_time: e.target.value };
                  onChange({ ...plan, schedules: newSchedules });
                }}
              />
            </div>
            <div className="form-group">
              <label>End Time</label>
              <input
                type="time"
                value={schedule.end_time}
                onChange={(e) => {
                  const newSchedules = [...plan.schedules];
                  newSchedules[i] = { ...schedule, end_time: e.target.value };
                  onChange({ ...plan, schedules: newSchedules });
                }}
              />
            </div>
          </div>
        </div>
      ))}

      <button onClick={addSchedule} className="btn-secondary">
        + Add Schedule
      </button>
    </section>
  );
};

// Placeholder step components — TODO: Full implementation in T-036

const StepApps: React.FC<{ plan: Plan; onChange: (p: Plan) => void }> = ({ plan, onChange }) => {
  const [processName, setProcessName] = useState("");
  const [action, setAction] = useState<"block" | "allow">("block");

  const addRule = () => {
    const trimmed = processName.trim();
    if (!trimmed) return;
    onChange({
      ...plan,
      app_rules: [
        ...plan.app_rules,
        { process_name: trimmed, action },
      ],
    });
    setProcessName("");
  };

  const removeRule = (index: number) => {
    onChange({
      ...plan,
      app_rules: plan.app_rules.filter((_, i) => i !== index),
    });
  };

  return (
    <section>
      <h2>App Rules</h2>
      <p>Add applications to block (or allow) during this plan's schedule.</p>

      <div className="form-row">
        <div className="form-group" style={{ flex: 1 }}>
          <label htmlFor="process-name">Process Name</label>
          <input
            id="process-name"
            type="text"
            value={processName}
            onChange={(e) => setProcessName(e.target.value)}
            placeholder="e.g., spotify.exe, Discord, com.game.app"
            onKeyDown={(e) => e.key === "Enter" && addRule()}
          />
        </div>
        <div className="form-group">
          <label htmlFor="app-action">Action</label>
          <select
            id="app-action"
            value={action}
            onChange={(e) => setAction(e.target.value as "block" | "allow")}
          >
            <option value="block">Block</option>
            <option value="allow">Allow</option>
          </select>
        </div>
        <button className="btn-secondary" onClick={addRule} style={{ alignSelf: "flex-end" }}>
          Add
        </button>
      </div>

      {plan.app_rules.length > 0 && (
        <table className="rules-table">
          <thead>
            <tr>
              <th>Process / App</th>
              <th>Action</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {plan.app_rules.map((rule, i) => (
              <tr key={i}>
                <td>{rule.process_name || rule.path_prefix || rule.bundle_id || "—"}</td>
                <td>
                  <span className={`badge badge-${rule.action}`}>
                    {rule.action}
                  </span>
                </td>
                <td>
                  <button className="btn-icon" onClick={() => removeRule(i)} aria-label="Remove rule">
                    ✕
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {plan.app_rules.length === 0 && (
        <p className="empty-state">No app rules yet. Add a process name above.</p>
      )}
    </section>
  );
};

const StepUrls: React.FC<{ plan: Plan; onChange: (p: Plan) => void }> = ({ plan, onChange }) => {
  const [domain, setDomain] = useState("");
  const [pathPattern, setPathPattern] = useState("");
  const [action, setAction] = useState<"block" | "allow">("block");

  const addRule = () => {
    const trimmedDomain = domain.trim().toLowerCase().replace(/^https?:\/\//, "").replace(/\/.*$/, "");
    if (!trimmedDomain) return;
    onChange({
      ...plan,
      url_rules: [
        ...plan.url_rules,
        {
          domain: trimmedDomain,
          path_pattern: pathPattern.trim() || undefined,
          action,
        },
      ],
    });
    setDomain("");
    setPathPattern("");
  };

  const removeRule = (index: number) => {
    onChange({
      ...plan,
      url_rules: plan.url_rules.filter((_, i) => i !== index),
    });
  };

  /** Common distraction sites for quick-add */
  const QUICK_ADD_DOMAINS = [
    "reddit.com", "twitter.com", "facebook.com", "instagram.com",
    "tiktok.com", "youtube.com", "twitch.tv", "netflix.com",
  ];

  const quickAdd = (d: string) => {
    if (plan.url_rules.some((r) => r.domain === d)) return;
    onChange({
      ...plan,
      url_rules: [...plan.url_rules, { domain: d, action: "block" }],
    });
  };

  return (
    <section>
      <h2>URL Rules</h2>
      <p>Add websites to block (or allow) during this plan's schedule.</p>

      {/* Quick-add popular sites */}
      <div className="quick-add">
        <span className="quick-add-label">Quick add:</span>
        {QUICK_ADD_DOMAINS.map((d) => (
          <button
            key={d}
            className={`chip ${plan.url_rules.some((r) => r.domain === d) ? "chip-active" : ""}`}
            onClick={() => quickAdd(d)}
            disabled={plan.url_rules.some((r) => r.domain === d)}
          >
            {d}
          </button>
        ))}
      </div>

      <div className="form-row">
        <div className="form-group" style={{ flex: 1 }}>
          <label htmlFor="domain-input">Domain</label>
          <input
            id="domain-input"
            type="text"
            value={domain}
            onChange={(e) => setDomain(e.target.value)}
            placeholder="e.g., reddit.com, *.social-media.com"
            onKeyDown={(e) => e.key === "Enter" && addRule()}
          />
        </div>
        <div className="form-group">
          <label htmlFor="path-pattern">Path (optional)</label>
          <input
            id="path-pattern"
            type="text"
            value={pathPattern}
            onChange={(e) => setPathPattern(e.target.value)}
            placeholder="e.g., /feed/*"
          />
        </div>
        <div className="form-group">
          <label htmlFor="url-action">Action</label>
          <select
            id="url-action"
            value={action}
            onChange={(e) => setAction(e.target.value as "block" | "allow")}
          >
            <option value="block">Block</option>
            <option value="allow">Allow</option>
          </select>
        </div>
        <button className="btn-secondary" onClick={addRule} style={{ alignSelf: "flex-end" }}>
          Add
        </button>
      </div>

      {plan.url_rules.length > 0 && (
        <table className="rules-table">
          <thead>
            <tr>
              <th>Domain</th>
              <th>Path</th>
              <th>Action</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {plan.url_rules.map((rule, i) => (
              <tr key={i}>
                <td>{rule.domain}</td>
                <td>{rule.path_pattern || "All paths"}</td>
                <td>
                  <span className={`badge badge-${rule.action}`}>
                    {rule.action}
                  </span>
                </td>
                <td>
                  <button className="btn-icon" onClick={() => removeRule(i)} aria-label="Remove rule">
                    ✕
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {plan.url_rules.length === 0 && (
        <p className="empty-state">No URL rules yet. Add a domain or use quick-add above.</p>
      )}
    </section>
  );
};

const StepQuotas: React.FC<{ plan: Plan; onChange: (p: Plan) => void }> = ({ plan, onChange }) => {
  const [target, setTarget] = useState("");
  const [targetType, setTargetType] = useState<"app" | "domain">("app");
  const [dailyMinutes, setDailyMinutes] = useState<number | "">(60);
  const [weeklyMinutes, setWeeklyMinutes] = useState<number | "">("");
  const [sessionMinutes, setSessionMinutes] = useState<number | "">("");

  const addQuota = () => {
    const trimmed = target.trim();
    if (!trimmed) return;

    const quota: Quota = {
      target: trimmed,
      target_type: targetType,
      daily_seconds: dailyMinutes ? Number(dailyMinutes) * 60 : undefined,
      weekly_seconds: weeklyMinutes ? Number(weeklyMinutes) * 60 : undefined,
      session_seconds: sessionMinutes ? Number(sessionMinutes) * 60 : undefined,
    };

    onChange({ ...plan, quotas: [...plan.quotas, quota] });
    setTarget("");
    setDailyMinutes(60);
    setWeeklyMinutes("");
    setSessionMinutes("");
  };

  const removeQuota = (index: number) => {
    onChange({ ...plan, quotas: plan.quotas.filter((_, i) => i !== index) });
  };

  const formatLimit = (seconds?: number): string => {
    if (!seconds) return "—";
    const mins = Math.floor(seconds / 60);
    if (mins >= 60) return `${Math.floor(mins / 60)}h ${mins % 60}m`;
    return `${mins}m`;
  };

  return (
    <section>
      <h2>Quotas</h2>
      <p>Set time limits for specific apps or websites. When the quota is exhausted, the target will be blocked for the rest of the period.</p>

      <div className="form-row">
        <div className="form-group" style={{ flex: 1 }}>
          <label htmlFor="quota-target">Target</label>
          <input
            id="quota-target"
            type="text"
            value={target}
            onChange={(e) => setTarget(e.target.value)}
            placeholder={targetType === "app" ? "e.g., spotify.exe" : "e.g., youtube.com"}
          />
        </div>
        <div className="form-group">
          <label htmlFor="quota-type">Type</label>
          <select
            id="quota-type"
            value={targetType}
            onChange={(e) => setTargetType(e.target.value as "app" | "domain")}
          >
            <option value="app">App</option>
            <option value="domain">Domain</option>
          </select>
        </div>
      </div>

      <div className="form-row">
        <div className="form-group">
          <label htmlFor="daily-limit">Daily Limit (minutes)</label>
          <input
            id="daily-limit"
            type="number"
            min={1}
            max={1440}
            value={dailyMinutes}
            onChange={(e) => setDailyMinutes(e.target.value ? parseInt(e.target.value) : "")}
            placeholder="e.g., 60"
          />
        </div>
        <div className="form-group">
          <label htmlFor="weekly-limit">Weekly Limit (minutes)</label>
          <input
            id="weekly-limit"
            type="number"
            min={1}
            max={10080}
            value={weeklyMinutes}
            onChange={(e) => setWeeklyMinutes(e.target.value ? parseInt(e.target.value) : "")}
            placeholder="optional"
          />
        </div>
        <div className="form-group">
          <label htmlFor="session-limit">Session Limit (minutes)</label>
          <input
            id="session-limit"
            type="number"
            min={1}
            max={480}
            value={sessionMinutes}
            onChange={(e) => setSessionMinutes(e.target.value ? parseInt(e.target.value) : "")}
            placeholder="optional"
          />
        </div>
        <button className="btn-secondary" onClick={addQuota} style={{ alignSelf: "flex-end" }}>
          Add
        </button>
      </div>

      {plan.quotas.length > 0 && (
        <table className="rules-table">
          <thead>
            <tr>
              <th>Target</th>
              <th>Type</th>
              <th>Daily</th>
              <th>Weekly</th>
              <th>Session</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {plan.quotas.map((q, i) => (
              <tr key={i}>
                <td>{q.target}</td>
                <td>{q.target_type}</td>
                <td>{formatLimit(q.daily_seconds)}</td>
                <td>{formatLimit(q.weekly_seconds)}</td>
                <td>{formatLimit(q.session_seconds)}</td>
                <td>
                  <button className="btn-icon" onClick={() => removeQuota(i)} aria-label="Remove quota">
                    ✕
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {plan.quotas.length === 0 && (
        <p className="empty-state">No quotas configured. Add a time limit above.</p>
      )}
    </section>
  );
};

const StepForcedMode: React.FC<{ plan: Plan; onChange: (p: Plan) => void }> = ({ plan, onChange }) => (
  <section>
    <h2>Forced Mode</h2>
    <p>Lock this plan so it cannot be modified or disabled until the timer expires.</p>
    <div className="form-group">
      <label>
        <input
          type="checkbox"
          checked={plan.forced_mode?.enabled ?? false}
          onChange={(e) =>
            onChange({
              ...plan,
              forced_mode: e.target.checked
                ? { enabled: true, duration_minutes: 60, password_protected: true }
                : null,
            })
          }
        />
        Enable Forced Mode
      </label>
    </div>
    {plan.forced_mode?.enabled && (
      <div className="form-group">
        <label>Duration (minutes)</label>
        <input
          type="number"
          min={1}
          max={1440}
          value={plan.forced_mode.duration_minutes}
          onChange={(e) =>
            onChange({
              ...plan,
              forced_mode: {
                ...plan.forced_mode!,
                duration_minutes: parseInt(e.target.value, 10) || 60,
              },
            })
          }
        />
      </div>
    )}
  </section>
);

const StepReview: React.FC<{ plan: Plan }> = ({ plan }) => (
  <section>
    <h2>Review Plan</h2>
    <dl className="review-summary">
      <dt>Name</dt>
      <dd>{plan.name || "(unnamed)"}</dd>
      <dt>Enabled</dt>
      <dd>{plan.enabled ? "Yes" : "No"}</dd>
      <dt>Schedules</dt>
      <dd>{plan.schedules.length} schedule(s)</dd>
      <dt>App Rules</dt>
      <dd>{plan.app_rules.length} rule(s)</dd>
      <dt>URL Rules</dt>
      <dd>{plan.url_rules.length} rule(s)</dd>
      <dt>Quotas</dt>
      <dd>{plan.quotas.length} quota(s)</dd>
      <dt>Forced Mode</dt>
      <dd>{plan.forced_mode?.enabled ? `${plan.forced_mode.duration_minutes} min` : "Off"}</dd>
    </dl>
  </section>
);

export default PlanWizard;
