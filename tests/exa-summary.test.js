'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const EXA = require('../src/exa-summary');
const PAJAK = require('../src/pajak-cluster');

const ROOT = path.resolve(__dirname, '..');

test('Exa AI Free-Tier Summarizer & DJP Spotlight Contracts', async (t) => {
  await t.test('isValidPublicHttpUrl rejects loopback, private, and non-HTTP URLs', () => {
    assert.equal(EXA.isValidPublicHttpUrl('https://news.ddtc.co.id/berita/coretax'), true);
    assert.equal(EXA.isValidPublicHttpUrl('http://127.0.0.1:8787/api/x'), false);
    assert.equal(EXA.isValidPublicHttpUrl('http://localhost/secret'), false);
    assert.equal(EXA.isValidPublicHttpUrl('http://192.168.1.1/admin'), false);
    assert.equal(EXA.isValidPublicHttpUrl('file:///etc/passwd'), false);
    assert.equal(EXA.isValidPublicHttpUrl('javascript:alert(1)'), false);
  });

  await t.test('summarize() calls Exa /contents with structured schema, caches hits, and deduplicates in-flight', async () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sitrep-exa-'));
    let calls = 0;
    const mockFetch = async (url, opt) => {
      calls++;
      assert.equal(url, 'https://api.exa.ai/contents');
      assert.equal(opt.headers['x-api-key'], 'test-exa-key');
      const body = JSON.parse(opt.body);
      assert.ok(Array.isArray(body.urls) && body.urls[0] === 'https://news.ddtc.co.id/coretax-update');
      assert.equal(body.summary.schema.type, 'object');
      await new Promise(r => setTimeout(r, 20));
      return {
        ok: true,
        status: 200,
        json: async () => ({
          results: [{
            url: 'https://news.ddtc.co.id/coretax-update',
            title: 'DJP Optimalisasi Coretax',
            summary: JSON.stringify({
              ringkasan: 'DJP melakukan pembaruan kapasitas server Coretax menjelang batas pelaporan SPT.',
              dampak_djp: 'Menurunkan risiko gagal login dan mempercepat penerbitan faktur pajak.',
              entitas_kunci: ['DJP', 'Coretax', 'SPT'],
              atensi: 'Pantau tren keluhan wajib pajak pada jam puncak.'
            })
          }]
        })
      };
    };

    const svc = EXA.createExaSummaryService({
      histDir: tmpDir,
      fetchFn: mockFetch,
      env: { EXA_API_KEY: 'test-exa-key', EXA_DAILY_CAP: '5' },
      persist: true,
    });

    const [r1, r2] = await Promise.all([
      svc.summarize({ url: 'https://news.ddtc.co.id/coretax-update', title: 'DJP Optimalisasi Coretax' }),
      svc.summarize({ url: 'https://news.ddtc.co.id/coretax-update', title: 'DJP Optimalisasi Coretax' }),
    ]);
    assert.equal(calls, 1, 'concurrent identical requests must share a single Exa call');
    assert.equal(r1.mode, 'exa');
    assert.equal(r1.ringkasan.includes('Coretax'), true);
    assert.deepEqual(r1.entitas_kunci, ['DJP', 'Coretax', 'SPT']);
    assert.equal(r2.cached, true);

    const r3 = await svc.summarize({ url: 'https://news.ddtc.co.id/coretax-update', title: 'DJP Optimalisasi Coretax' });
    assert.equal(calls, 1, 'subsequent call must hit cache');
    assert.equal(r3.cached, true);
    assert.equal(svc.status().dailyUsed, 1);
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  await t.test('enforces EXA_DAILY_CAP and falls back cleanly when key is missing or 429 occurs', async () => {
    const noKeySvc = EXA.createExaSummaryService({ env: {}, persist: false });
    const fb = await noKeySvc.summarize({
      url: 'https://finance.detik.com/pajak-digital',
      title: 'Setoran Pajak Digital Tembus Rp60,1 Triliun',
      desc: 'Direktorat Jenderal Pajak mencatat penerimaan pajak usaha ekonomi digital mencapai Rp60,1 triliun hingga akhir kuartal.'
    });
    assert.equal(fb.ok, true);
    assert.equal(fb.mode, 'fallback');
    assert.equal(fb.exaConfigured, false);
    assert.ok(fb.entitas_kunci.some(e => /Rp60,1/i.test(e)));

    let calls = 0;
    const capSvc = EXA.createExaSummaryService({
      env: { EXA_API_KEY: 'k', EXA_DAILY_CAP: '1' },
      persist: false,
      fetchFn: async () => {
        calls++;
        return {
          ok: true,
          status: 200,
          json: async () => ({
            results: [{ url: 'https://kontan.co.id/a', title: 'A', summary: JSON.stringify({ ringkasan: 'OK A', dampak_djp: 'D', entitas_kunci: ['APBN'], atensi: 'P' }) }]
          })
        };
      }
    });
    const first = await capSvc.summarize({ url: 'https://kontan.co.id/a', title: 'A' });
    const second = await capSvc.summarize({ url: 'https://kontan.co.id/b', title: 'B', desc: 'Realisasi APBN dan penerimaan negara naik 8 persen.' });
    assert.equal(first.mode, 'exa');
    assert.equal(second.mode, 'fallback');
    assert.equal(second.fallbackReason, 'DAILY_CAP');
    assert.equal(calls, 1);
  });

  await t.test('Keuangan Negara items (APBN, Kemenkeu, PNBP, Bea Cukai) classify as Pajak Nasional', () => {
    const samples = [
      'Menkeu Laporkan Realisasi Penerimaan Negara dan Defisit APBN Terkendali',
      'DJBC Bea Cukai dan DJP Sinergi Dongkrak Penerimaan PNBP dan Pajak',
      'Kemenkeu Terbitkan PMK Baru Soal Kebijakan Fiskal dan SBN',
    ];
    for (const s of samples) {
      assert.equal(PAJAK.isTaxText(s), true, `Expected tax/fiscal relevant: ${s}`);
      assert.equal(PAJAK.classifyTaxScope(s), 'nasional', `Expected nasional scope: ${s}`);
    }
  });

  await t.test('compiled public/index.html enables DJP-first default active tabs and wires Exa UI', () => {
    const html = fs.readFileSync(path.join(ROOT, 'public', 'index.html'), 'utf8');
    assert.ok(html.includes("catFilter = 'pajak'"), 'News Wire must default active tab to pajak');
    assert.ok(html.includes("let pjScope = 'nasional'"), 'Klaster Pajak must default active scope to nasional');
    assert.ok(html.includes('/api/pajak/summarize'), 'Client must wire /api/pajak/summarize');
    assert.ok(html.includes('/api/pajak/brief'), 'Client must wire /api/pajak/brief');
    assert.ok(html.includes('id="pj-brief-btn"'), 'Klaster Pajak must include Briefing Exa AI button');
    assert.ok(html.includes('data-lead-exa="1"'), 'Hero lead card must include Ringkas Exa AI trigger');
  });
});
