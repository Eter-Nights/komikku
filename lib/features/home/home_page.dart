import 'package:flutter/material.dart';

import 'package:komikku/features/category/category_show_page.dart';
import 'package:komikku/features/home/promote_show_page.dart';
import 'package:komikku/features/home/serialization_show_page.dart';
import 'package:komikku/features/search/search_page.dart';
import 'package:komikku/features/search/search_show_page.dart';
import 'package:komikku/rust/api.dart';
import 'package:komikku/rust/service/model.dart';
import 'package:komikku/shared/widgets/album_row.dart';
import 'package:komikku/shared/widgets/section_header.dart';

/// 首页：展示 promote 接口的推荐分组（只保留 promote / category_id / not_in_category_id 三种类型）。
class HomePage extends StatefulWidget {
  const HomePage({super.key});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  /// 首页只展示这三种类型的分组（library / novels 等类型不展示）
  static const _supportedTypes = {
    'promote',
    'category_id',
    'not_in_category_id',
  };

  /// 连载更新分组的 id：标题显示「周 * 连载更新」，查看更多进入连载更新页
  static const _serializationSectionId = 26;

  List<PromoteSectionInfo>? _sections;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() => _error = null);
    try {
      final sections = await getPromote();
      if (!mounted) return;
      setState(() {
        _sections = sections
            .where((s) => _supportedTypes.contains(s.sectionType))
            .toList();
      });
    } catch (e) {
      if (!mounted) return;
      setState(() => _error = e.toString());
    }
  }

  void _openSearch() {
    Navigator.of(context)
        .push(MaterialPageRoute(builder: (_) => const SearchPage()));
  }

  /// 「查看更多」按分组类型分流到对应列表页；连载更新分组（id=26）进入专用连载页
  void _openMore(PromoteSectionInfo section) {
    if (section.id.toInt() == _serializationSectionId) {
      Navigator.of(
        context,
      ).push(MaterialPageRoute(builder: (_) => const SerializationShowPage()));
      return;
    }
    final page = switch (section.sectionType) {
      'promote' => PromoteShowPage(id: section.id, title: section.title),
      'category_id' => CategoryShowPage(
        slug: section.slug,
        title: section.title,
      ),
      'not_in_category_id' => SearchShowPage(
        keyword: section.slug.isEmpty ? section.title : section.slug,
      ),
      _ => null,
    };
    if (page == null) return;
    Navigator.of(context).push(MaterialPageRoute(builder: (_) => page));
  }

  void _onAlbumTap(AlbumBriefInfo album) {
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(
        SnackBar(
          content: Text('「${album.name}」详情页开发中'),
          duration: const Duration(seconds: 1),
        ),
      );
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Scaffold(
      appBar: AppBar(
        title: const Text(
          'Komikku',
          style: TextStyle(fontWeight: FontWeight.bold),
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.search),
            tooltip: '搜索',
            onPressed: _openSearch,
          ),
        ],
      ),
      body: RefreshIndicator(
        onRefresh: _load,
        child: ListView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.only(top: 8, bottom: 24),
          children: [
            if (_sections == null && _error == null)
              const Padding(
                padding: EdgeInsets.all(32),
                child: Center(child: CircularProgressIndicator()),
              )
            else if (_error != null && _sections == null)
              Padding(
                padding: const EdgeInsets.all(32),
                child: Center(
                  child: Column(
                    children: [
                      Text('栏目加载失败'),
                      const SizedBox(height: 8),
                      Text(
                        _error!,
                        textAlign: TextAlign.center,
                        style: Theme.of(context).textTheme.bodySmall
                            ?.copyWith(color: scheme.error),
                      ),
                      const SizedBox(height: 8),
                      FilledButton.tonal(
                        onPressed: _load,
                        child: const Text('重试'),
                      ),
                    ],
                  ),
                ),
              )
            else
              for (final s in _sections!) ...[
                _PromoteSection(
                  section: s,
                  onAlbumTap: _onAlbumTap,
                  onMore: () => _openMore(s),
                ),
                const SizedBox(height: 8),
              ],
          ],
        ),
      ),
    );
  }
}

/// 首页推荐栏目：一个 promote 分组一栏，直接展示分组内嵌的专辑。
/// 纯展示组件：标题 + 横向专辑行。「查看更多」与点击行为由上层（首页）决定。
class _PromoteSection extends StatelessWidget {
  const _PromoteSection({required this.section, this.onAlbumTap, this.onMore});

  /// 连载更新分组的 id：标题显示「周 * 连载更新」
  static const _serializationSectionId = 26;

  final PromoteSectionInfo section;
  final void Function(AlbumBriefInfo)? onAlbumTap;
  final VoidCallback? onMore;

  /// 今天周几的中文标签（周一~周日）
  String _todayWeekdayLabel() {
    const labels = ['周一', '周二', '周三', '周四', '周五', '周六', '周日'];
    return labels[DateTime.now().weekday - 1];
  }

  /// 连载更新分组的标题：周 * 连载更新（* 为今天周几）
  String _serializationTitle() => '${_todayWeekdayLabel()}连载更新';

  @override
  Widget build(BuildContext context) {
    // 连载更新分组（id=26）内部根据今天周几生成标题，其余分组用默认标题
    final title = section.id.toInt() == _serializationSectionId
        ? _serializationTitle()
        : section.title;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SectionHeader(title: title, onMore: onMore),
        AlbumRow(items: section.content, onTap: onAlbumTap),
      ],
    );
  }
}
