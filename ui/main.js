const { invoke } = window.__TAURI__.core;

// ─── Tab navigation ────────────────────────────────────────────────────

document.querySelectorAll('.nav-btn').forEach(btn => {
  btn.addEventListener('click', () => {
    document.querySelectorAll('.nav-btn').forEach(b => b.classList.remove('active'));
    document.querySelectorAll('.tab-content').forEach(t => t.classList.remove('active'));
    btn.classList.add('active');
    document.getElementById('tab-' + btn.dataset.tab).classList.add('active');
  });
});

// ─── Helpers ───────────────────────────────────────────────────────────

function showMsg(id, text, type) {
  const el = document.getElementById(id);
  el.textContent = text;
  el.className = 'msg show ' + type;
  setTimeout(() => { el.className = 'msg'; }, 8000);
}

function scoreClass(score) {
  if (score >= 70) return 'score-high';
  if (score >= 40) return 'score-mid';
  return 'score-low';
}

function escapeHtml(s) {
  if (!s) return '';
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function truncate(s, n) {
  if (!s) return '';
  return s.length > n ? s.substring(0, n) + '...' : s;
}

// ─── Dashboard ─────────────────────────────────────────────────────────

async function refreshStats() {
  try {
    const stats = await invoke('get_db_stats');
    document.getElementById('stat-jobs').textContent = stats.jobs;
    document.getElementById('stat-candidates').textContent = stats.candidates;
    document.getElementById('stat-skills').textContent = stats.skills_kb;
    document.getElementById('stat-cooccurrence').textContent = stats.skill_cooccurrence;
    document.getElementById('stat-matches').textContent = stats.matches;
  } catch (e) {
    showMsg('dashboard-msg', 'Error: ' + e, 'error');
  }
}

async function exportRag() {
  try {
    showMsg('dashboard-msg', 'Exporting...', 'info');
    const result = await invoke('export_rag');
    showMsg('dashboard-msg', result, 'success');
  } catch (e) {
    showMsg('dashboard-msg', 'Error: ' + e, 'error');
  }
}

// ─── Pipeline ──────────────────────────────────────────────────────────

async function runPipeline() {
  const btn = document.getElementById('btn-pipeline');
  btn.disabled = true;
  btn.innerHTML = 'Running... <span class="loading"></span>';
  showMsg('pipeline-msg', 'Pipeline running...', 'info');

  try {
    const result = await invoke('run_full_pipeline', {
      force: document.getElementById('pipeline-force').checked,
      useRag: document.getElementById('pipeline-rag').checked,
      minScore: parseFloat(document.getElementById('pipeline-min-score').value),
      topN: parseInt(document.getElementById('pipeline-top-n').value),
    });

    const box = document.getElementById('pipeline-result');
    box.classList.remove('hidden');

    if (result.success) {
      showMsg('pipeline-msg', result.message, 'success');
      box.innerHTML = `<strong>Pipeline Results</strong>
Jobs processed:       ${result.jobs_processed}
Candidates processed: ${result.candidates_processed}
Skills learned:       ${result.skills_learned}
Matches found:        ${result.matches_found}
Execution time:       ${result.execution_time.toFixed(2)}s`;
      refreshStats();
    } else {
      showMsg('pipeline-msg', result.message, 'error');
      box.textContent = result.message;
    }
  } catch (e) {
    showMsg('pipeline-msg', 'Error: ' + e, 'error');
  }

  btn.disabled = false;
  btn.textContent = 'Run Full Pipeline';
}

async function ingestJobs(force) {
  try {
    showMsg('pipeline-msg', 'Ingesting jobs...', 'info');
    const r = await invoke('ingest_jobs', { force });
    showMsg('pipeline-msg', `${r.records} jobs, ${r.skills_extracted} skills: ${r.message}`, r.success ? 'success' : 'error');
    refreshStats();
  } catch (e) {
    showMsg('pipeline-msg', 'Error: ' + e, 'error');
  }
}

async function ingestCvs(force) {
  try {
    showMsg('pipeline-msg', 'Ingesting CVs...', 'info');
    const r = await invoke('ingest_cvs', { force });
    showMsg('pipeline-msg', `${r.records} candidates, ${r.skills_extracted} skills: ${r.message}`, r.success ? 'success' : 'error');
    refreshStats();
  } catch (e) {
    showMsg('pipeline-msg', 'Error: ' + e, 'error');
  }
}

// ─── Matches ───────────────────────────────────────────────────────────

async function loadMatches() {
  try {
    const matches = await invoke('get_matches');
    document.getElementById('matches-count').textContent = `${matches.length} matches loaded`;

    const tbody = document.querySelector('#matches-table tbody');
    tbody.innerHTML = matches.map((m, i) => `<tr>
      <td>${i + 1}</td>
      <td class="${scoreClass(m.overall_score)}">${m.overall_score.toFixed(1)}%</td>
      <td title="${escapeHtml(m.job_id)}">${escapeHtml(truncate(m.job_title, 40))}</td>
      <td>${escapeHtml(m.candidate_name)}</td>
      <td class="${scoreClass(m.skill_score)}">${m.skill_score.toFixed(1)}%</td>
      <td title="${escapeHtml(m.matched_skills)}">${escapeHtml(truncate(m.matched_skills, 50))}</td>
      <td title="${escapeHtml(m.missing_skills)}">${escapeHtml(truncate(m.missing_skills, 50))}</td>
    </tr>`).join('');
  } catch (e) {
    document.getElementById('matches-count').textContent = 'Error: ' + e;
  }
}

// ─── Jobs ──────────────────────────────────────────────────────────────

async function loadJobs() {
  try {
    const jobs = await invoke('get_jobs');
    document.getElementById('jobs-count').textContent = `${jobs.length} jobs loaded`;

    const tbody = document.querySelector('#jobs-table tbody');
    tbody.innerHTML = jobs.map(j => `<tr>
      <td>${escapeHtml(j.job_id)}</td>
      <td>${escapeHtml(truncate(j.title, 50))}</td>
      <td>${escapeHtml(j.buyer)}</td>
      <td>${escapeHtml(j.location)}</td>
      <td>${escapeHtml(j.status)}</td>
      <td title="${escapeHtml(j.skills_raw)}">${escapeHtml(truncate(j.skills_raw, 60))}</td>
    </tr>`).join('');
  } catch (e) {
    document.getElementById('jobs-count').textContent = 'Error: ' + e;
  }
}

// ─── Candidates ────────────────────────────────────────────────────────

async function loadCandidates() {
  try {
    const candidates = await invoke('get_candidates');
    document.getElementById('candidates-count').textContent = `${candidates.length} candidates loaded`;

    const tbody = document.querySelector('#candidates-table tbody');
    tbody.innerHTML = candidates.map(c => `<tr>
      <td>${escapeHtml(c.name)}</td>
      <td>${escapeHtml(c.email)}</td>
      <td>${escapeHtml(c.location)}</td>
      <td>${c.experience_years || '0'} years</td>
      <td title="${escapeHtml(c.skills_raw)}">${escapeHtml(truncate(c.skills_raw, 60))}</td>
    </tr>`).join('');
  } catch (e) {
    document.getElementById('candidates-count').textContent = 'Error: ' + e;
  }
}

// ─── Skills ────────────────────────────────────────────────────────────

async function loadSkills() {
  try {
    const skills = await invoke('get_skills');
    document.getElementById('skills-count').textContent = `${skills.length} skills loaded`;

    const tbody = document.querySelector('#skills-table tbody');
    tbody.innerHTML = skills.map(s => `<tr>
      <td>${escapeHtml(s.skill_name)}</td>
      <td>${escapeHtml(s.canonical_name)}</td>
      <td><span class="skill-tag ${s.category === 'soft_skills' ? 'soft' : 'tech'}">${escapeHtml(s.category)}</span></td>
      <td>${s.frequency}</td>
    </tr>`).join('');
  } catch (e) {
    document.getElementById('skills-count').textContent = 'Error: ' + e;
  }
}

// ─── Analyzer ──────────────────────────────────────────────────────────

async function analyzeText() {
  const text = document.getElementById('analyze-text').value;
  if (!text.trim()) return;

  try {
    const result = await invoke('analyze_text', { text });
    const box = document.getElementById('analyze-result');
    box.classList.remove('hidden');

    let html = `<strong>Found ${result.skills.length} skills:</strong>\n\n`;

    if (result.categories.technical && result.categories.technical.length > 0) {
      html += `<strong>Technical:</strong>\n<div class="skill-tags">${result.categories.technical.map(s => `<span class="skill-tag tech">${s}</span>`).join('')}</div>\n`;
    }
    if (result.categories.soft && result.categories.soft.length > 0) {
      html += `<strong>Soft Skills:</strong>\n<div class="skill-tags">${result.categories.soft.map(s => `<span class="skill-tag soft">${s}</span>`).join('')}</div>\n`;
    }
    if (result.categories.other && result.categories.other.length > 0) {
      html += `<strong>Other:</strong>\n<div class="skill-tags">${result.categories.other.map(s => `<span class="skill-tag">${s}</span>`).join('')}</div>`;
    }

    box.innerHTML = html;
  } catch (e) {
    document.getElementById('analyze-result').textContent = 'Error: ' + e;
    document.getElementById('analyze-result').classList.remove('hidden');
  }
}

async function quickMatch() {
  const candidateSkills = document.getElementById('match-candidate-skills').value;
  const jobSkills = document.getElementById('match-job-skills').value;
  if (!candidateSkills.trim() || !jobSkills.trim()) return;

  try {
    const result = await invoke('match_single', {
      candidateSkills,
      jobSkills,
    });
    const box = document.getElementById('match-result');
    box.classList.remove('hidden');

    const score = result.overall_score;
    const barColor = score >= 70 ? 'var(--success)' : score >= 40 ? 'var(--warning)' : 'var(--danger)';

    box.innerHTML = `<strong>Match Score: <span class="${scoreClass(score)}">${score.toFixed(1)}%</span></strong>
<div class="score-bar"><div class="score-bar-fill" style="width:${score}%;background:${barColor}"></div></div>

<strong>Skill Score:</strong> ${result.skill_score.toFixed(1)}%

<strong>Matched:</strong>
<div class="skill-tags">${(result.matched_skills || []).map(s => `<span class="skill-tag tech">${s}</span>`).join('') || 'None'}</div>

<strong>Missing:</strong>
<div class="skill-tags">${(result.missing_skills || []).map(s => `<span class="skill-tag">${s}</span>`).join('') || 'None'}</div>`;
  } catch (e) {
    document.getElementById('match-result').textContent = 'Error: ' + e;
    document.getElementById('match-result').classList.remove('hidden');
  }
}

// ─── Init ──────────────────────────────────────────────────────────────

document.addEventListener('DOMContentLoaded', () => {
  refreshStats();
});
