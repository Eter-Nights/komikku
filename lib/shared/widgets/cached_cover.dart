import 'dart:io';

import 'package:flutter/material.dart';

import 'package:komikku/rust/api.dart';

/// 封面加载组件：`getCover` 返回磁盘路径，`Image.file` 显示。
/// 后端已做磁盘缓存，这里再做一层 Future 缓存避免重复请求。
class CachedCover extends StatefulWidget {
  const CachedCover({super.key, required this.imageName, this.borderRadius});

  /// 封面文件名，如 `{album_id}_3x4.jpg`（缩略图）或 `{album_id}.jpg`（原图）
  final String imageName;
  final BorderRadius? borderRadius;

  @override
  State<CachedCover> createState() => _CachedCoverState();
}

class _CachedCoverState extends State<CachedCover> {
  late Future<String> _future;

  @override
  void initState() {
    super.initState();
    _future = getCover(imageName: widget.imageName);
  }

  @override
  void didUpdateWidget(CachedCover oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.imageName != widget.imageName) {
      _future = getCover(imageName: widget.imageName);
    }
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final radius = widget.borderRadius ?? BorderRadius.circular(12);

    return ClipRRect(
      borderRadius: radius,
      child: FutureBuilder<String>(
        future: _future,
        builder: (context, snapshot) {
          if (snapshot.hasError) {
            return _Placeholder(
              scheme: scheme,
              icon: Icons.broken_image_outlined,
            );
          }
          if (!snapshot.hasData) {
            return _Placeholder(
              scheme: scheme,
              icon: Icons.image_outlined,
              loading: true,
            );
          }
          return Image.file(
            File(snapshot.data!),
            fit: BoxFit.cover,
            gaplessPlayback: true,
            errorBuilder: (_, _, _) =>
                _Placeholder(scheme: scheme, icon: Icons.broken_image_outlined),
            frameBuilder: (context, child, frame, wasSyncLoaded) {
              if (wasSyncLoaded || frame != null) return child;
              return _FadeIn(child: child);
            },
          );
        },
      ),
    );
  }
}

class _Placeholder extends StatelessWidget {
  const _Placeholder({
    required this.scheme,
    required this.icon,
    this.loading = false,
  });

  final ColorScheme scheme;
  final IconData icon;
  final bool loading;

  @override
  Widget build(BuildContext context) {
    return Container(
      color: scheme.surfaceContainerHighest,
      alignment: Alignment.center,
      child: loading
          ? const SizedBox(
              width: 20,
              height: 20,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          : Icon(icon, color: scheme.onSurfaceVariant.withValues(alpha: 0.5)),
    );
  }
}

class _FadeIn extends StatelessWidget {
  const _FadeIn({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return TweenAnimationBuilder<double>(
      tween: Tween(begin: 0, end: 1),
      duration: const Duration(milliseconds: 250),
      builder: (context, value, c) => Opacity(opacity: value, child: c),
      child: child,
    );
  }
}
