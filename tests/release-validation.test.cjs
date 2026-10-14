const assert = require('node:assert/strict');
const { test } = require('node:test');
const { validateRun, versionPattern } = require('../.github/scripts/prepare-release.cjs');

const valid = {
  workflow_id: 7, status: 'completed', conclusion: 'success', event: 'push',
  head_branch: 'main', head_repository: { full_name: 'owner/rordle' }, head_sha: 'a'.repeat(40),
};
const validate = run => validateRun(run, 7, 'owner/rordle', 'main');

test('accepts successful push and manual CI runs on the default branch', () => {
  validate(valid);
  validate({ ...valid, event: 'workflow_dispatch' });
});

for (const [description, override] of [
  ['failed CI', { conclusion: 'failure' }],
  ['unfinished CI', { status: 'in_progress' }],
  ['the wrong workflow', { workflow_id: 8 }],
  ['a pull request', { event: 'pull_request' }],
  ['a feature branch', { head_branch: 'feature' }],
  ['a fork', { head_repository: { full_name: 'fork/rordle' } }],
  ['an invalid source SHA', { head_sha: 'main' }],
]) {
  test(`rejects ${description}`, () => assert.throws(() => validate({ ...valid, ...override })));
}

test('release versions are stable numeric tags suitable for URL paths', () => {
  for (const version of ['v0.1.0', 'v1.20.300']) assert.equal(versionPattern.test(version), true);
  for (const version of ['v01.0.0', 'latest', '../v1.0.0', 'v1.0.0-beta', 'v1.0.0\n']) {
    assert.equal(versionPattern.test(version), false, version);
  }
});
