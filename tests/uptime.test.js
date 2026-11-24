'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const UPTIME = require('../src/uptime-service');
const ROOT = path.resolve(__dirname, '..');

test('DJP & Kemenkeu Website Uptime Monitor (isdownapi.com)', async (t) => {
  await t.test('monitors all 4 required official portals', () => {
    const domains = UPTIME.DJP_PORTALS.map(p => p.domain);
    assert.deepEqual(domains, [
      'pajak.go.id',
      'coretaxdjp.pajak.go.id',
      'djponline.pajak.go.id',
      'kemenkeu.go.id',
    ]);
  });

  await t.test('classifies up, slow (>=2500ms), and down accurately with caching and history', async () => {
    let calls = 0;
    const responses = {
      'pajak.go.id': { success: true, domain: 'pajak.go.id', status: 'up', http_code: 200, response_time_ms: 2731, ip: '103.28.106.146' },
      'coretaxdjp.pajak.go.id': { success: true, domain: 'coretaxdjp.pajak.go.id', status: 'up', http_code: 200, response_time_ms: 775, ip: '43.242.132.12' },
      'djponline.pajak.go.id': { success: true, domain: 'djponline.pajak.go.id', status: 'down', http_code: null, response_time_ms: null, error: 'SSL_ERROR_SYSCALL' },
      'kemenkeu.go.id': { success: true, domain: 'kemenkeu.go.id', status: 'up', http_code: 200, response_time_ms: 1120, ip: '103.165.248.88' },
    };
    const svc = UPTIME.createUptimeService({
      ttlMs: 60000,
      fetchFn: async (url) => {
        calls++;
        const dom = new URL(url).searchParams.get('domain');
        return { ok: true, status: 200, json: async () => responses[dom] };
      }
    });

    const r1 = await svc.checkAll(false);
    assert.equal(calls, 4);
    assert.equal(r1.ok, true);
    assert.equal(r1.upCount, 3);
    assert.equal(r1.slowCount, 1);
    assert.equal(r1.downCount, 1);
    assert.equal(r1.items.find(x => x.id === 'pajak').state, 'slow');
    assert.equal(r1.items.find(x => x.id === 'coretax').state, 'up');
    assert.equal(r1.items.find(x => x.id === 'djponline').state, 'down');
    assert.equal(r1.items.find(x => x.id === 'kemenkeu').state, 'up');

    const r2 = await svc.checkAll(false);
    assert.equal(calls, 4, 'second call within TTL must hit cache');
    assert.equal(r2.cached, true);

    const r3 = await svc.checkAll(true);
    assert.equal(calls, 8, 'force refresh must re-probe all 4 domains');
    assert.equal(r3.items.find(x => x.id === 'coretax').history.length, 2);
  });

  await t.test('compiled public/index.html wires #djp-uptime, #pj-uptime, and /api/pajak/uptime', () => {
    const html = fs.readFileSync(path.join(ROOT, 'public', 'index.html'), 'utf8');
    assert.ok(html.includes('id="djp-uptime"'), 'Main console must render #djp-uptime status bar');
    assert.ok(html.includes('id="pj-uptime"'), 'Klaster Pajak must render #pj-uptime status bar');
    assert.ok(html.includes('/api/pajak/uptime'), 'Client must poll /api/pajak/uptime');
  });
});
