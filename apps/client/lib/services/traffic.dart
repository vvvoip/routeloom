int countValue(dynamic value) =>
    value is num && value.isFinite && value >= 0 ? value.toInt() : 0;
String textValue(dynamic value, [String fallback = '']) =>
    value is String && value.length <= 2048 ? value : fallback;

class FlowEntry {
  FlowEntry(Map<String, dynamic> json)
    : id = countValue(json['id']),
      host = textValue(json['host']),
      source = textValue(json['source']),
      protocol = textValue(json['protocol']),
      port = countValue(json['port']),
      address = textValue(json['address']),
      route = textValue(json['route'], 'pending'),
      rule = textValue(json['rule']),
      ruleLine = countValue(json['rule_line']),
      status = textValue(json['status']),
      reason = textValue(json['reason']),
      startedAt = countValue(json['started_at']),
      durationMs = countValue(json['duration_ms']),
      connectMs = json['connect_ms'] is num
          ? countValue(json['connect_ms'])
          : null,
      upload = countValue(json['upload']),
      download = countValue(json['download']);
  final int id, port, ruleLine, startedAt, durationMs, upload, download;
  final int? connectMs;
  final String host, source, protocol, address, route, rule, status, reason;
  bool get isOpen => status == 'active' || status == 'connecting';
  bool matches(
    String search,
    String protocolFilter,
    String routeFilter,
    bool openOnly,
  ) {
    final text = (host + ' ' + address + ' ' + source + ' ' + rule)
        .toLowerCase();
    return text.contains(search.trim().toLowerCase()) &&
        (protocolFilter == 'all' || protocol == protocolFilter) &&
        (routeFilter == 'all' || route == routeFilter) &&
        (!openOnly || isOpen);
  }
}

class TrafficSnapshot {
  TrafficSnapshot(Map<String, dynamic> json)
    : session = countValue(json['session']),
      mode = textValue(json['mode'], 'auto'),
      active = countValue(json['active']),
      upload = countValue(json['upload']),
      download = countValue(json['download']),
      diagnostic = json['diagnostic'] is Map<String, dynamic>
          ? json['diagnostic'] as Map<String, dynamic>
          : {},
      connections = ((json['connections'] as List?) ?? [])
          .take(256)
          .whereType<Map<String, dynamic>>()
          .map(FlowEntry.new)
          .toList() {
    if (json['schema'] != 1 || !['auto', 'proxy', 'direct'].contains(mode))
      throw const FormatException('Unsupported traffic snapshot');
  }
  final int session, active, upload, download;
  final String mode;
  final List<FlowEntry> connections;
  final Map<String, dynamic> diagnostic;
}

String formatTraffic(num value) {
  if (value < 1024) return value.round().toString() + ' B';
  if (value < 1024 * 1024) return (value / 1024).toStringAsFixed(1) + ' KiB';
  if (value < 1024 * 1024 * 1024)
    return (value / (1024 * 1024)).toStringAsFixed(1) + ' MiB';
  return (value / (1024 * 1024 * 1024)).toStringAsFixed(2) + ' GiB';
}
