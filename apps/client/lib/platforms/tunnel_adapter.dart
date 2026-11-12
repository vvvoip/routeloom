import 'package:flutter/foundation.dart';

enum TunnelState { unavailable, disconnected, connecting, connected, disconnecting, failed }

/// Real adapters must emit connected only after authenticated transport,
/// routes, and DNS have all been established.
abstract interface class TunnelAdapter {
  TargetPlatform get platform;
  Stream<TunnelState> get states;
  Future<void> connect();
  Future<void> disconnect();
}
class PendingTunnelAdapter implements TunnelAdapter {
  PendingTunnelAdapter(this.platform);
  @override
  final TargetPlatform platform;
  @override
  Stream<TunnelState> get states => Stream.value(TunnelState.unavailable);
  @override
  Future<void> connect() async {
    throw UnsupportedError('Native VPN adapter is not implemented');
  }
  @override
  Future<void> disconnect() async {}
}
const clientPlatforms = [
  TargetPlatform.windows, TargetPlatform.macOS, TargetPlatform.linux,
  TargetPlatform.android, TargetPlatform.iOS,
];
String platformName(TargetPlatform platform) => switch (platform) {
  TargetPlatform.windows => 'Windows', TargetPlatform.macOS => 'macOS',
  TargetPlatform.linux => 'Linux', TargetPlatform.android => 'Android',
  TargetPlatform.iOS => 'iOS / iPadOS', TargetPlatform.fuchsia => 'Unsupported',
};
