import 'dart:async';
import 'package:flutter/material.dart';
import '../l10n/generated/app_localizations.dart';
import '../platforms/tunnel_adapter.dart';
import '../services/traffic.dart';

String modeLabel(AppLocalizations t, String mode) => switch (mode) {
  'direct' => t.modeDirect,
  'proxy' => t.modeProxy,
  _ => t.modeAuto,
};
String routeLabel(AppLocalizations t, String route) => switch (route) {
  'direct' => t.routeDirect,
  'proxy' => t.routeProxy,
  'block' => t.routeBlock,
  _ => t.routePending,
};

class ModePicker extends StatelessWidget {
  const ModePicker({
    super.key,
    required this.mode,
    required this.enabled,
    required this.onChanged,
  });
  final String mode;
  final bool enabled;
  final ValueChanged<String> onChanged;
  @override
  Widget build(BuildContext context) {
    final t = AppLocalizations.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(t.routingMode, style: Theme.of(context).textTheme.titleSmall),
        DropdownButton<String>(
          key: const Key('routing-mode'),
          isExpanded: true,
          value: mode,
          items: [
            for (final value in ['auto', 'proxy', 'direct'])
              DropdownMenuItem(value: value, child: Text(modeLabel(t, value))),
          ],
          onChanged: enabled
              ? (value) {
                  if (value != null) onChanged(value);
                }
              : null,
        ),
        Text(t.modeHint, style: Theme.of(context).textTheme.bodySmall),
        if (!enabled)
          Text(t.modeChangeHint, style: Theme.of(context).textTheme.bodySmall),
      ],
    );
  }
}

class TrafficPanel extends StatefulWidget {
  const TrafficPanel({
    super.key,
    required this.adapter,
    required this.active,
    required this.connected,
  });
  final TunnelAdapter adapter;
  final bool active, connected;
  @override
  State<TrafficPanel> createState() => _TrafficPanelState();
}

class _TrafficPanelState extends State<TrafficPanel> {
  Timer? _timer;
  TrafficSnapshot? _snapshot;
  DateTime? _lastTick;
  bool _polling = false,
      _paused = false,
      _stale = false,
      _busy = false,
      _openOnly = false;
  String _search = '', _protocol = 'all', _route = 'all';
  int _limit = 50;
  double _upSpeed = 0, _downSpeed = 0;
  @override
  void initState() {
    super.initState();
    _poll();
    _timer = Timer.periodic(const Duration(seconds: 1), (_) => _poll());
  }

  @override
  void didUpdateWidget(covariant TrafficPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.active && !oldWidget.active) {
      _lastTick = null;
      _poll();
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  Future<void> _poll() async {
    if (_polling || _paused || !widget.active) return;
    _polling = true;
    try {
      final raw = await widget.adapter.snapshot();
      if (!mounted) return;
      if (raw == null) {
        setState(() => _stale = true);
        return;
      }
      final next = TrafficSnapshot(raw), now = DateTime.now(), old = _snapshot;
      final elapsed = _lastTick == null
          ? 0
          : now.difference(_lastTick!).inMilliseconds;
      var up = 0.0, down = 0.0;
      if (old != null && old.session == next.session && elapsed > 0) {
        up = ((next.upload - old.upload).clamp(0, 1 << 62)) * 1000 / elapsed;
        down =
            ((next.download - old.download).clamp(0, 1 << 62)) * 1000 / elapsed;
      }
      setState(() {
        _snapshot = next;
        _stale = false;
        _lastTick = now;
        _upSpeed = up;
        _downSpeed = down;
      });
    } catch (_) {
      if (mounted) setState(() => _stale = true);
    } finally {
      _polling = false;
    }
  }

  Future<void> _command(String command) async {
    if (_busy) return;
    setState(() => _busy = true);
    try {
      await widget.adapter.monitorCommand(command);
      await _poll();
    } catch (_) {
      if (mounted)
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(AppLocalizations.of(context).operationFailed)),
        );
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Widget _metric(String label, String value) => SizedBox(
    width: 145,
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(label, style: Theme.of(context).textTheme.bodySmall),
        Text(value, style: Theme.of(context).textTheme.titleMedium),
      ],
    ),
  );
  Widget _detail(String label, String value) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 3),
    child: SelectableText(label + ': ' + value),
  );
  Widget _diagnostics(AppLocalizations t) {
    final d = _snapshot?.diagnostic ?? {};
    final status = textValue(d['status'], 'idle');
    final running = status == 'running';
    final checked = countValue(d['checked_at']);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(t.diagnostics, style: Theme.of(context).textTheme.titleLarge),
            const SizedBox(height: 8),
            Text(t.diagnosticHint),
            const SizedBox(height: 12),
            Align(
              alignment: Alignment.centerLeft,
              child: FilledButton.icon(
                key: const Key('run-diagnostic'),
                onPressed: widget.connected && !_paused && !_busy && !running
                    ? () => _command('diagnose')
                    : null,
                icon: running
                    ? const SizedBox(
                        width: 16,
                        height: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.network_check),
                label: Text(running ? t.diagnosticRunning : t.runDiagnostic),
              ),
            ),
            const SizedBox(height: 10),
            if (status == 'idle') Text(t.diagnosticIdle),
            if (status == 'failed') ...[
              Text(t.diagnosticFailed),
              _detail(
                t.currentMode,
                modeLabel(t, textValue(d['mode'], 'auto')),
              ),
              if (d['route'] != null)
                _detail(
                  t.diagnosticRoute,
                  routeLabel(t, textValue(d['route'])),
                ),
            ],
            if (status == 'success') ...[
              Text(
                t.diagnosticSuccess,
                style: const TextStyle(fontWeight: FontWeight.bold),
              ),
              _detail(
                t.currentMode,
                modeLabel(t, textValue(d['mode'], 'auto')),
              ),
              _detail(t.diagnosticRoute, routeLabel(t, textValue(d['route']))),
              _detail(t.exitIp, textValue(d['exit_ip'], t.notAvailable)),
              _detail(t.serverIp, textValue(d['server_ip'], t.notAvailable)),
              _detail(
                t.matchesServer,
                d['matches_server'] == true ? t.yesValue : t.noValue,
              ),
              _detail(
                t.latency,
                countValue(d['latency_ms']).toString() + ' ' + t.milliseconds,
              ),
            ],
            if (checked > 0)
              _detail(
                t.checkedAt,
                DateTime.fromMillisecondsSinceEpoch(
                  checked,
                ).toLocal().toString().split('.').first,
              ),
            const SizedBox(height: 8),
            Text(
              t.diagnosticScope,
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ],
        ),
      ),
    );
  }

  Widget _row(AppLocalizations t, FlowEntry flow) {
    final status = switch (flow.status) {
      'active' => t.flowActive,
      'connecting' => t.flowConnecting,
      'failed' => t.flowFailed,
      'blocked' => t.flowBlocked,
      _ => t.flowClosed,
    };
    final reason = switch (flow.reason) {
      'blocked' => t.flowBlocked,
      'engine_unavailable' => t.reasonUnavailable,
      'resolution_failed' => t.reasonResolution,
      'connect_failed' => t.reasonConnect,
      'timeout' => t.reasonTimeout,
      'io_error' => t.reasonIo,
      _ => flow.reason,
    };
    final color = switch (flow.route) {
      'proxy' => Colors.indigo,
      'direct' => Colors.teal,
      'block' => Colors.red,
      _ => Colors.blueGrey,
    };
    return Card(
      key: ValueKey('flow-' + flow.id.toString()),
      child: ExpansionTile(
        title: Text(
          flow.host + ':' + flow.port.toString(),
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
        ),
        subtitle: Text(
          flow.protocol.toUpperCase() +
              ' · ' +
              status +
              '\n↑ ' +
              formatTraffic(flow.upload) +
              '   ↓ ' +
              formatTraffic(flow.download),
        ),
        trailing: Container(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
          decoration: BoxDecoration(
            color: color.withValues(alpha: .1),
            borderRadius: BorderRadius.circular(8),
          ),
          child: Text(
            routeLabel(t, flow.route),
            style: TextStyle(color: color),
          ),
        ),
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _detail(
                  t.flowTarget,
                  flow.address.isEmpty ? t.notAvailable : flow.address,
                ),
                _detail(t.flowSource, flow.source),
                _detail(
                  t.flowRule,
                  flow.ruleLine > 0
                      ? '#' + flow.ruleLine.toString() + ' ' + flow.rule
                      : flow.route == 'pending'
                      ? t.notAvailable
                      : t.flowModeOverride,
                ),
                _detail(
                  t.flowDuration,
                  (flow.durationMs / 1000).toStringAsFixed(1) + ' ' + t.seconds,
                ),
                if (flow.connectMs != null)
                  _detail(
                    t.flowConnectTime,
                    flow.connectMs.toString() + ' ' + t.milliseconds,
                  ),
                if (reason.isNotEmpty) _detail(t.flowReason, reason),
              ],
            ),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final t = AppLocalizations.of(context), s = _snapshot;
    final entries = (s?.connections ?? <FlowEntry>[])
        .where((f) => f.matches(_search, _protocol, _route, _openOnly))
        .toList();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (!widget.active) Text(t.monitorOffline),
        if (widget.active && _stale) Text(t.monitorStale),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  t.capturedTraffic,
                  style: Theme.of(context).textTheme.titleLarge,
                ),
                if (s != null)
                  Text(t.currentMode + ': ' + modeLabel(t, s.mode)),
                const SizedBox(height: 14),
                Wrap(
                  spacing: 20,
                  runSpacing: 12,
                  children: [
                    _metric(t.activeConnections, (s?.active ?? 0).toString()),
                    _metric(t.uploadTraffic, formatTraffic(s?.upload ?? 0)),
                    _metric(t.downloadTraffic, formatTraffic(s?.download ?? 0)),
                    _metric(
                      t.speedUpload,
                      formatTraffic(widget.active ? _upSpeed : 0),
                    ),
                    _metric(
                      t.speedDownload,
                      formatTraffic(widget.active ? _downSpeed : 0),
                    ),
                  ],
                ),
                const SizedBox(height: 10),
                Text(
                  t.payloadNote,
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ],
            ),
          ),
        ),
        _diagnostics(t),
        const SizedBox(height: 16),
        Text(t.connections, style: Theme.of(context).textTheme.titleLarge),
        const SizedBox(height: 6),
        Text(t.monitorHint, style: Theme.of(context).textTheme.bodySmall),
        const SizedBox(height: 12),
        TextField(
          key: const Key('flow-search'),
          decoration: InputDecoration(
            prefixIcon: const Icon(Icons.search),
            labelText: t.monitorSearch,
            border: const OutlineInputBorder(),
          ),
          onChanged: (value) => setState(() {
            _search = value;
            _limit = 50;
          }),
        ),
        Wrap(
          spacing: 16,
          runSpacing: 4,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            DropdownButton<String>(
              value: _protocol,
              items: [
                DropdownMenuItem(value: 'all', child: Text(t.allProtocols)),
                const DropdownMenuItem(value: 'tcp', child: Text('TCP')),
                const DropdownMenuItem(value: 'udp', child: Text('UDP')),
              ],
              onChanged: (v) => setState(() => _protocol = v ?? 'all'),
            ),
            DropdownButton<String>(
              value: _route,
              items: [
                DropdownMenuItem(value: 'all', child: Text(t.allRoutes)),
                for (final v in ['proxy', 'direct', 'block'])
                  DropdownMenuItem(value: v, child: Text(routeLabel(t, v))),
              ],
              onChanged: (v) => setState(() => _route = v ?? 'all'),
            ),
            FilterChip(
              label: Text(t.onlyActive),
              selected: _openOnly,
              onSelected: (v) => setState(() => _openOnly = v),
            ),
            TextButton.icon(
              onPressed: () {
                setState(() {
                  _paused = !_paused;
                  _lastTick = null;
                });
                if (!_paused) _poll();
              },
              icon: Icon(_paused ? Icons.play_arrow : Icons.pause),
              label: Text(_paused ? t.resumeView : t.pauseView),
            ),
            TextButton.icon(
              onPressed: widget.active && !_busy && !_paused
                  ? () => _command('clear')
                  : null,
              icon: const Icon(Icons.clear_all),
              label: Text(t.clearHistory),
            ),
          ],
        ),
        if (entries.isEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 24),
            child: Text(t.monitorEmpty),
          ),
        for (final flow in entries.take(_limit)) _row(t, flow),
        if (entries.length > _limit)
          TextButton(
            onPressed: () => setState(() => _limit += 50),
            child: Text(t.showMore),
          ),
      ],
    );
  }
}
