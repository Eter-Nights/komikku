import 'package:flutter/material.dart';

import 'package:komikku/features/search/search_show_page.dart';

/// 搜索页：AppBar 内嵌搜索框。
class SearchPage extends StatefulWidget {
  const SearchPage({super.key});

  @override
  State<SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends State<SearchPage> {
  final _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _search(String raw) {
    final keyword = raw.trim();
    if (keyword.isEmpty) return;
    Navigator.of(
      context,
    ).push(MaterialPageRoute(builder: (_) => SearchShowPage(keyword: keyword)));
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        titleSpacing: 0,
        title: TextField(
          controller: _controller,
          autofocus: true,
          textInputAction: TextInputAction.search,
          onSubmitted: _search,
          decoration: const InputDecoration(
            hintText: '输入关键字搜索',
            border: InputBorder.none,
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => _search(_controller.text),
            child: const Text('搜索'),
          ),
        ],
      ),
      body: const SizedBox.shrink(),
    );
  }
}
