import 'dart:convert';
import 'package:flutter/services.dart';

class RoutingPresets {
  static Future<String> chinaAndLan() async {
    final policy = await rootBundle.loadString('assets/china-lan.rules');
    if (utf8.encode(policy).length > 16384 ||
        !policy.trimRight().endsWith('final proxy:default')) {
      throw const FormatException('Invalid bundled routing preset');
    }
    return policy;
  }
}
