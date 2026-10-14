const fs = require('node:fs');

const versionPattern = /^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?![\s\S])/;

function validateRun(run, workflowId, repository, branch) {
  if (run.workflow_id !== workflowId || run.status !== 'completed' || run.conclusion !== 'success') {
    throw new Error('Choose a completed, successful CI workflow run.');
  }
  if (!['push', 'workflow_dispatch'].includes(run.event) || run.head_branch !== branch ||
      run.head_repository?.full_name !== repository) {
    throw new Error('Only CI runs from this repository’s default branch can be released.');
  }
  if (!/^[0-9a-f]{40}$/.test(run.head_sha)) throw new Error('CI run has an invalid source SHA.');
}

async function prepare({ github, context, core }) {
  const tag = (process.env.RELEASE_VERSION || '').trim();
  const runId = (process.env.CI_RUN_ID || '').trim();
  if (!versionPattern.test(tag)) throw new Error('Use a version such as v0.1.0 (vMAJOR.MINOR.PATCH).');
  if (!/^[1-9][0-9]*$/.test(runId) || !Number.isSafeInteger(Number(runId))) {
    throw new Error('CI run ID must be a positive integer.');
  }
  const repo = context.repo;
  const { data: repository } = await github.rest.repos.get(repo);
  if (context.eventName !== 'workflow_dispatch' || context.ref !== `refs/heads/${repository.default_branch}`) {
    throw new Error('Run the release workflow manually from the default branch.');
  }
  const { data: workflow } = await github.rest.actions.getWorkflow({ ...repo, workflow_id: 'rust.yml' });
  const { data: run } = await github.rest.actions.getWorkflowRun({ ...repo, run_id: Number(runId) });
  validateRun(run, workflow.id, repository.full_name, repository.default_branch);
  try {
    await github.rest.git.getRef({ ...repo, ref: `tags/${tag}` });
    throw new Error(`Tag ${tag} already exists; choose a new release version.`);
  } catch (error) {
    if (error.status !== 404) throw error;
  }
  const artifacts = await github.paginate(github.rest.actions.listWorkflowRunArtifacts, {
    ...repo, run_id: Number(runId), per_page: 100,
  });
  const matches = artifacts.filter(a => a.name === `rordle-wasm-${run.head_sha}` && !a.expired);
  if (matches.length !== 1) throw new Error('The CI bundle is missing or expired. Rerun CI first.');
  const releases = await github.paginate(github.rest.repos.listReleases, { ...repo, per_page: 100 });
  const history = releases.filter(release => !release.draft && !release.prerelease &&
    release.assets.some(asset => asset.name === 'rordle-web.zip'));
  for (const release of history) {
    if (!versionPattern.test(release.tag_name)) throw new Error(`Unexpected web release tag: ${release.tag_name}`);
    if (release.tag_name === tag) throw new Error(`Release ${tag} already exists.`);
  }
  fs.writeFileSync('history-tags.txt', history.map(release => release.tag_name).join('\n') + (history.length ? '\n' : ''));
  core.setOutput('sha', run.head_sha);
  core.setOutput('tag', tag);
  core.setOutput('run_id', runId);
  core.setOutput('artifact_id', String(matches[0].id));
  await core.summary.addHeading('Approved WASM candidate')
    .addLink(`CI run ${runId}`, run.html_url)
    .addRaw(`\n\nSource: \`${run.head_sha}\`\n\nVersion: \`${tag}\`\n`)
    .write();
}

module.exports = prepare;
module.exports.validateRun = validateRun;
module.exports.versionPattern = versionPattern;
