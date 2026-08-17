import 'package:flutter/material.dart';

import 'package:komikku/shared/widgets/album_grid.dart';
import 'package:komikku/shared/widgets/tag_bar.dart';
import 'package:komikku/rust/api.dart';
import 'package:komikku/rust/comic_source/jmcomic/client.dart';
import 'package:komikku/rust/service/model.dart';

/// 分类列表页：子标签 + CategorySort 排序 + 分页网格。
/// 点击子标签在当前页面切换过滤条件，不创建新页面。
class CategoryShowPage extends StatefulWidget {
  const CategoryShowPage({
    super.key,
    required this.slug,
    required this.title,
    this.subCategories = const [],
  });

  final String slug;
  final String title;

  /// 主分类下的子标签（点击后按 主slug_子slug 过滤）
  final List<CategorySubInfo> subCategories;

  @override
  State<CategoryShowPage> createState() => _CategoryShowPageState();
}

class _CategoryShowPageState extends State<CategoryShowPage> {
  static const _sorts = [
    CategorySort.latest,
    CategorySort.like,
    CategorySort.totalRanking,
    CategorySort.monthRanking,
  ];

  static const _sortLabels = ['最新', '最多喜欢', '总排名', '月排名'];

  final _gridKey = GlobalKey<AlbumGridState>();
  late String _slug;
  String _selectedSubSlug = '';
  int _sortIndex = 0;

  @override
  void initState() {
    super.initState();
    _slug = widget.slug;
  }

  /// 选择子标签；空字符串表示「全部」，回到主分类不拼接
  void _selectSub(String subSlug) {
    setState(() {
      _slug = subSlug.isEmpty ? widget.slug : '${widget.slug}_$subSlug';
      _selectedSubSlug = subSlug;
      _sortIndex = 0;
    });
    _gridKey.currentState?.reset();
  }

  void _changeSort(int i) {
    if (i == _sortIndex) return;
    setState(() {
      _sortIndex = i;
    });
    _gridKey.currentState?.reset();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text(widget.title)),
      body: Column(
        children: [
          TagBar<String>(
            wrap: true,
            items: [
              (label: '全部', value: ''),
              for (final sub in widget.subCategories)
                (label: sub.name, value: sub.slug),
            ],
            selected: _selectedSubSlug,
            onSelected: _selectSub,
          ),
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
              loadPage: (page) => getCategoriesFilter(
                category: _slug,
                page: page,
                sort: _sorts[_sortIndex],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
