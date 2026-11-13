import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:routeloom/main.dart';
import 'package:routeloom/platforms/tunnel_adapter.dart';

void main() {
  testWidgets('connect requires an imported profile', (tester) async {
    await tester.pumpWidget(const RouteLoomApp());
    await tester.pumpAndSettle();
    expect(find.text('Not connected'), findsOneWidget);
    final button = tester.widget<FilledButton>(
      find.byKey(const Key('connect-button')),
    );
    expect(button.onPressed, isNull);
    expect(find.text('Developer preview'), findsOneWidget);
    await tester.pumpWidget(const SizedBox());
  });
  testWidgets('language can switch to Chinese and back', (tester) async {
    await tester.pumpWidget(const RouteLoomApp());
    await tester.pumpAndSettle();
    await tester.tap(find.text('Settings'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('language-selector')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('简体中文').last);
    await tester.pumpAndSettle();
    expect(find.text('设置'), findsOneWidget);
    expect(find.text('隐私设计'), findsOneWidget);
    await tester.tap(find.byKey(const Key('language-selector')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('English').last);
    await tester.pumpAndSettle();
    expect(find.text('Privacy by design'), findsOneWidget);
    await tester.pumpWidget(const SizedBox());
  });
  for (final platform in clientPlatforms) {
    testWidgets('interface renders on ' + platformName(platform), (
      tester,
    ) async {
      debugDefaultTargetPlatformOverride = platform;
      addTearDown(() => debugDefaultTargetPlatformOverride = null);
      await tester.pumpWidget(const RouteLoomApp());
      await tester.pumpAndSettle();
      expect(find.text('RouteLoom'), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
    });
  }
}
