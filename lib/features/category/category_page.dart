import 'package:flutter/material.dart';

import 'package:komikku/features/category/category_show_page.dart';
import 'package:komikku/features/search/search_show_page.dart';
import 'package:komikku/rust/api.dart';
import 'package:komikku/rust/service/model.dart';

/// 分类页：展示分类列表和分类词条。
class CategoryPage extends StatefulWidget {
  const CategoryPage({super.key});

  @override
  State<CategoryPage> createState() => _CategoryPageState();
}

class _CategoryPageState extends State<CategoryPage> {
  CategoryInfo? _info;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() => _error = null);
    try {
      final info = await getCategories();
      if (!mounted) return;
      setState(() => _info = info);
    } catch (e) {
      if (!mounted) return;
      setState(() => _error = e.toString());
    }
  }

  void _openCategory(CategoryItemInfo category) {
    Navigator.of(context).push(
      MaterialPageRoute(
        builder: (_) => CategoryShowPage(
          slug: category.slug,
          title: category.name,
          subCategories: category.subCategories,
        ),
      ),
    );
  }

  void _search(String keyword) {
    Navigator.of(
      context,
    ).push(MaterialPageRoute(builder: (_) => SearchShowPage(keyword: keyword)));
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('分类')),
      body: RefreshIndicator(onRefresh: _load, child: _buildBody()),
    );
  }

  Widget _buildBody() {
    if (_info == null && _error == null) {
      return ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        children: [
          const SizedBox(height: 300),
          const Center(child: CircularProgressIndicator()),
        ],
      );
    }
    if (_error != null && _info == null) {
      return ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.all(32),
        children: [
          const SizedBox(height: 120),
          const Center(child: Text('分类加载失败')),
          const SizedBox(height: 8),
          Center(child: Text(_error!, textAlign: TextAlign.center)),
          const SizedBox(height: 8),
          Center(
            child: FilledButton.tonal(
              onPressed: _load,
              child: const Text('重试'),
            ),
          ),
        ],
      );
    }

    final info = _info!;
    return ListView(
      physics: const AlwaysScrollableScrollPhysics(),
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
      children: [
        if (info.categories.isNotEmpty) ...[
          const _SectionTitle(title: '更多分类'),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              for (final category in info.categories)
                ActionChip(
                  label: Text(category.name),
                  onPressed: () => _openCategory(category),
                ),
            ],
          ),
        ],
        for (final block in info.blocks)
          _BlockSection(block: block, onWord: _search),
        if (info.categories.isEmpty && info.blocks.isEmpty)
          const Center(child: Text('暂无分类内容')),
      ],
    );
  }
}

class _SectionTitle extends StatelessWidget {
  const _SectionTitle({required this.title});

  final String title;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: 10),
    child: Text(
      title,
      style: Theme.of(context).textTheme.titleMedium
          ?.copyWith(fontWeight: FontWeight.bold),
    ),
  );
}

class _BlockSection extends StatelessWidget {
  const _BlockSection({required this.block, required this.onWord});

  final CategoryBlockInfo block;
  final ValueChanged<String> onWord;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(top: 24),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _SectionTitle(title: block.title),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            for (final word in block.content)
              ActionChip(label: Text(word), onPressed: () => onWord(word)),
          ],
        ),
      ],
    ),
  );
}
