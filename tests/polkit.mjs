// SPDX-License-Identifier: MPL-2.0
// Exercise the actual shipped policy without requiring a running polkit daemon.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
let rule;
vm.runInNewContext(fs.readFileSync(new URL('../polkit/49-cosmic-nightlight.rules', import.meta.url), 'utf8'), {
  polkit: { Result: { YES: 'yes' }, addRule(fn) { rule = fn; } },
});
const paths = ['/usr/bin/cosmic-nightlight-helper', '/usr/local/bin/cosmic-nightlight-helper', '/var/usrlocal/bin/cosmic-nightlight-helper'];
let checks = 0;
for (const path of [...paths, '/tmp/cosmic-nightlight-helper', '/usr/bin/sh', paths[0] + '-other']) {
  for (const group of ['wheel', 'sudo', 'users']) {
    for (const local of [true, false]) {
      for (const active of [true, false]) {
        for (const id of ['org.freedesktop.policykit.exec', 'other.action']) {
          const result = rule({ id, lookup: () => path }, { local, active, isInGroup: g => g === group });
          const allowed = paths.includes(path) && group !== 'users' && local && active && id === 'org.freedesktop.policykit.exec';
          assert.equal(result, allowed ? 'yes' : undefined);
          checks++;
        }
      }
    }
  }
}
console.log(`${checks} polkit authorization cases passed`);
