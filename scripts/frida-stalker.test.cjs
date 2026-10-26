const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');
const vm = require('node:vm');

// Model NativePointer arithmetic independently of JavaScript's lossy Number type.
class Pointer {
    constructor(value) {
        this.value = BigInt(value);
    }

    add(offset) {
        return new Pointer(this.value + BigInt(offset));
    }

    compare(other) {
        return this.value < other.value ? -1 : this.value > other.value ? 1 : 0;
    }

    toString() {
        return `0x${this.value.toString(16)}`;
    }
}

function collector() {
    const module = { path: '/isolated/fixture', base: new Pointer('0xffff000000000000'), size: 65536 };
    let receive;
    let finalEvents = [];
    const context = vm.createContext({
        rpc: { exports: {} },
        Frida: { version: 'offline-fixture' },
        Process: { enumerateModules: () => [module], enumerateThreads: () => [{ id: 7 }] },
        Stalker: {
            follow(_thread, options) { receive = options.onReceive; },
            unfollow() {},
            flush() { receive(finalEvents); },
            parse(events) { return events; },
        },
    });
    const source = fs.readFileSync(path.join(__dirname, 'frida-stalker.js'), 'utf8');
    vm.runInContext(source, context);
    context.rpc.exports.start(module.path, 7, 'instructions');
    return {
        base: module.base,
        receive(events) { receive(events); },
        finish(events = []) {
            finalEvents = events;
            // Normalize objects crossing the VM boundary for structural assertions.
            return JSON.parse(JSON.stringify(context.rpc.exports.stop()));
        },
    };
}

test('preserves 64-bit addresses, inbound calls and events drained at stop', () => {
    const capture = collector();
    const base = capture.base;
    const external = new Pointer('0x1000');
    capture.receive([
        ['exec', base.add(1)],
        ['exec', base.add(1)],
        ['call', external, base.add(2), 0],
        ['call', base.add(3), external, 0],
        ['exec', external],
        ['exec', base.add(65536)],
    ]);
    const result = capture.finish([['block', base.add(16), base.add(32)]]);
    assert.equal(result.complete, true);
    assert.deepEqual(result.events, [
        { kind: 'instruction', address: '0xffff000000000001', thread_id: 7, count: 2 },
        { kind: 'call', address: '0x1000', thread_id: 7, count: 1, target: '0xffff000000000002' },
        { kind: 'call', address: '0xffff000000000003', thread_id: 7, count: 1, target: '0x1000' },
        { kind: 'block', address: '0xffff000000000010', thread_id: 7, count: 1 },
    ]);
});

test('retains bounded partial evidence after overflow or callback failure', () => {
    const capture = collector();
    const events = Array.from({ length: 10001 }, (_, index) => ['exec', capture.base.add(index)]);
    capture.receive(events);
    const result = capture.finish();
    assert.equal(result.complete, false);
    assert.equal(result.events.length, 10000);

    const failed = collector();
    failed.receive([['exec', failed.base]]);
    failed.receive([['exec', null]]);
    const partial = failed.finish();
    assert.equal(partial.complete, false);
    assert.equal(partial.events.length, 1);
    assert.match(partial.error, /TypeError/);
});
