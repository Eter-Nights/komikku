import 'package:flutter/material.dart';

import 'package:komikku/shared/widgets/album_card.dart';
import 'package:komikku/rust/service/model.dart';

/// 分页网格：触底加载下一页，支持排序变化后 reset 重新加载。
class AlbumGrid extends StatefulWidget {
  const AlbumGrid({
    super.key,
    required this.loadPage,
    this.desiredCellWidth = 132,
    this.onTotalChanged,
    this.unknownTotal = false,
  });

  /// 按页码加载数据（返回结构与搜索一致）
  final Future<SearchInfo> Function(int page) loadPage;

  /// 自适应模式下每张卡片的目标宽度（桌面宽屏自动增加列数）
  final double desiredCellWidth;

  /// 加载完成时回调总数（用于页面标题显示「共 xxx 部」）
  final ValueChanged<int>? onTotalChanged;

  /// 接口无 total 字段时置 true（如 serialization），触底条件改为「本页返回非空」
  final bool unknownTotal;

  @override
  State<AlbumGrid> createState() => AlbumGridState();
}

class AlbumGridState extends State<AlbumGrid> {
  final _items = <AlbumBriefInfo>[];
  final _scroll = ScrollController();
  int _page = 1;
  bool _loading = false;
  bool _hasMore = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _scroll.addListener(_onScroll);
    _load();
  }

  @override
  void dispose() {
    _scroll.dispose();
    super.dispose();
  }

  void _onScroll() {
    if (_scroll.position.pixels >= _scroll.position.maxScrollExtent - 300) {
      _load();
    }
  }

  Future<void> _load() async {
    if (_loading || !_hasMore) return;
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final info = await widget.loadPage(_page);
      if (!mounted) return;
      final total = info.total.toInt();
      setState(() {
        _items.addAll(info.content);
        widget.onTotalChanged?.call(total);
        _hasMore = widget.unknownTotal
            ? info.content.isNotEmpty
            : info.content.isNotEmpty && _items.length < total;
        _page++;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        // 加载失败时停止继续加载，避免底部转圈永远不消失
        _hasMore = false;
      });
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  /// 排序等变化后重置并重新加载第一页
  void reset() {
    setState(() {
      _items.clear();
      _page = 1;
      _hasMore = true;
      _error = null;
    });
    _load();
  }

  @override
  Widget build(BuildContext context) {
    if (_error != null && _items.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text('加载失败，请检查网络或代理设置'),
            TextButton(onPressed: _load, child: const Text('重试')),
          ],
        ),
      );
    }
    if (_items.isEmpty && _loading) {
      return LayoutBuilder(
        builder: (context, constraints) => GridView.builder(
          padding: const EdgeInsets.all(16),
          gridDelegate: _gridDelegate(constraints.maxWidth),
          itemCount: 9,
          itemBuilder: (context, i) => _GridSkeleton(),
        ),
      );
    }

    return LayoutBuilder(
      builder: (context, constraints) => GridView.builder(
        controller: _scroll,
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.all(16),
        gridDelegate: _gridDelegate(constraints.maxWidth),
        itemCount: _items.length + (_hasMore ? 1 : 0),
        itemBuilder: (context, i) {
          if (i >= _items.length) {
            return const Center(
              child: Padding(
                padding: EdgeInsets.all(8),
                child: SizedBox(
                  width: 20,
                  height: 20,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              ),
            );
          }
          return AlbumCard(album: _items[i]);
        },
      ),
    );
  }

  SliverGridDelegate _gridDelegate(double maxWidth) {
    final cols = (maxWidth / widget.desiredCellWidth).floor().clamp(2, 10);
    return SliverGridDelegateWithFixedCrossAxisCount(
      crossAxisCount: cols.toInt(),
      mainAxisSpacing: 16,
      crossAxisSpacing: 12,
      childAspectRatio: 0.55,
    );
  }
}

class _GridSkeleton extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final box = BoxDecoration(
      color: scheme.surfaceContainerHighest,
      borderRadius: BorderRadius.circular(12),
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Expanded(
          child: Container(width: double.infinity, decoration: box),
        ),
        const SizedBox(height: 8),
        Container(width: double.infinity, height: 12, decoration: box),
        const SizedBox(height: 6),
        Container(width: 80, height: 10, decoration: box),
      ],
    );
  }
}
