import 'dart:io';
import '../platforms/tunnel_adapter.dart';

class Protection {
  static Future<void> forgetProfile(TunnelProfile profile) async {
    if (Platform.isAndroid || Platform.isIOS || Platform.isMacOS) {
      await NativeTunnelAdapter.channel.invokeMethod<void>(
        'forgetProfile',
        profile.json,
      );
    }
  }

  static Future<bool> nativeStatus() async =>
      await NativeTunnelAdapter.channel.invokeMethod<bool>('guardStatus') ??
      false;
  static Future<void> settings() =>
      NativeTunnelAdapter.channel.invokeMethod<void>('guardSettings');
  static Future<void> unlock() async {
    if (Platform.isAndroid) return settings();
    if (Platform.isIOS || Platform.isMacOS)
      return NativeTunnelAdapter.channel.invokeMethod<void>('unlock');
    final binary =
        Platform.environment['ROUTELOOM_VPN_BINARY'] ??
        File(Platform.resolvedExecutable).parent.path +
            Platform.pathSeparator +
            'routeloom-vpn' +
            (Platform.isWindows ? '.exe' : '');
    final command = desktopTunnelCommand(binary, linux: Platform.isLinux);
    final child = await Process.start(command.first, command.sublist(1));
    child.stdout.drain<void>();
    child.stderr.drain<void>();
    child.stdin.writeln('unlock');
    await child.stdin.close();
    if (await child.exitCode.timeout(const Duration(seconds: 60)) != 0)
      throw StateError('Unlock failed');
  }
}
