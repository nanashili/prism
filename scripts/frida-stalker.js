// Loaded only by trace-frida.py after explicit authorization for an isolated target.
// Addresses stay as NativePointer/string values to avoid JavaScript integer rounding.
let capture = null;

function inModule(address, module) {
    return address.compare(module.base) >= 0 &&
        address.compare(module.base.add(module.size)) < 0;
}

function recordEvents(buffer) {
    try {
        aggregateEvents(buffer);
    } catch (error) {
        // Retain prior observations, but never report a failed callback as complete.
        capture.failure = String(error);
    }
}

function aggregateEvents(buffer) {
    for (const event of Stalker.parse(buffer, { annotate: true, stringify: false })) {
        const [kind, address, target] = event;
        if (!inModule(address, capture.module) &&
            !(kind === 'call' && inModule(target, capture.module))) {
            continue;
        }

        const observation = {
            kind: kind === 'exec' ? 'instruction' : kind,
            address: address.toString(),
            thread_id: capture.thread,
            count: 1,
        };
        if (kind === 'call') {
            observation.target = target.toString();
        }
        const key = `${kind}:${observation.address}:${observation.target || ''}`;
        const previous = capture.observations.get(key);
        if (previous) {
            if (previous.count < 1000000000) {
                previous.count += 1;
            } else {
                capture.truncated = true;
            }
        } else if (capture.observations.size < 10000) {
            capture.observations.set(key, observation);
        } else {
            capture.truncated = true;
        }
    }
}

rpc.exports = {
    start(modulePath, thread, mode) {
        if (capture !== null) {
            throw new Error('A capture is already active');
        }
        const module = Process.enumerateModules().find(module => module.path === modulePath);
        if (!module) {
            throw new Error('The exact target module path is not loaded in this process');
        }
        if (!Process.enumerateThreads().some(candidate => candidate.id === thread)) {
            throw new Error('The selected thread does not exist');
        }
        capture = { module, thread, observations: new Map(), truncated: false, failure: null };
        Stalker.follow(thread, {
            events: { call: true, ret: false, exec: mode === 'instructions',
                block: mode === 'blocks', compile: false },
            onReceive: recordEvents,
        });
        return { runtime_base: module.base.toString(), size: module.size, version: Frida.version };
    },

    stop() {
        if (capture === null) {
            throw new Error('No capture is active');
        }
        Stalker.unfollow(capture.thread);
        Stalker.flush();
        // Flush invokes onReceive before the RPC returns. Keep capture alive for
        // any final callback until script unload; the host then detaches the session.
        return {
            events: Array.from(capture.observations.values()),
            complete: !capture.truncated && capture.failure === null,
            error: capture.failure,
        };
    },
};
