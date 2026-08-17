import 'dart:io';

import 'package:flutter/material.dart';

import 'package:komikku/app.dart';
import 'package:komikku/rust/api.dart';
import 'package:komikku/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();

  // 数据目录：缓存 DB + 图片落盘（先用系统临时目录，后续接入 path_provider）
  final dataDir = '${Directory.systemTemp.path}/komikku';
  try {
    await initRust(path: dataDir);
  } catch (e) {
    debugPrint('init_rust 失败: $e');
  }

  runApp(const KomikkuApp());
}
