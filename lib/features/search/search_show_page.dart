import 'package:flutter/material.dart';

import 'package:komikku/shared/widgets/album_grid.dart';
import 'package:komikku/shared/widgets/tag_bar.dart';
import 'package:komikku/rust/api.dart';
import 'package:komikku/rust/comic_source/jmcomic/client.dart';

/// 搜索结果页：SearchSort 排序 + 分页网格。
/// 由搜索框提交或搜索页词条点击进入。
class SearchShowPage extends StatefulWidget {
  const SearchShowPage({super.key, required this.keyword});

  final String keyword;

  @override
  State<SearchShowPage> createState() => _SearchShowPageState();
}

class _SearchShowPageState extends State<SearchShowPage> {
  static const _sorts = [
    SearchSort.latest,
    SearchSort.view,
    SearchSort.picture,
    SearchSort.like,
  ];

  static const _sortLabels = ['最新', '最多点击', '最多图片', '最多喜欢'];

  final _gridKey = GlobalKey<AlbumGridState>();
  int _sortIndex = 0;
  int? _total;

  void _changeSort(int i) {
    if (i == _sortIndex) return;
    setState(() {
      _sortIndex = i;
      _total = null;
    });
    _gridKey.currentState?.reset();
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.keyword),
        actions: [
          if (_total != null)
            Padding(
              padding: const EdgeInsets.only(right: 16),
              child: Center(
                child: Text(
                  '共 $_total 部',
                  style: Theme.of(context).textTheme.bodySmall
                      ?.copyWith(color: scheme.onSurfaceVariant),
                ),
              ),
            ),
        ],
      ),
      body: Column(
        children: [
          TagBar<int>(
            items: [
              for (var i = 0; i < _sortLabels.length; i++)
                (label: _sortLabels[i], value: i),
            ],
            selected: _sortIndex,
            onSelected: _changeSort,
          ),
          Expanded(
            child: AlbumGrid(
              key: _gridKey,
              loadPage: (page) => search(
                keyword: widget.keyword,
                page: page,
                sort: _sorts[_sortIndex],
              ),
              onTotalChanged: (t) {
                if (mounted && _total != t) setState(() => _total = t);
              },
            ),
          ),
        ],
      ),
    );
  }
}
