import 'package:flutter/material.dart';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

import 'package:komikku/shared/widgets/album_grid.dart';
import 'package:komikku/rust/api.dart';
import 'package:komikku/rust/service/model.dart';

/// 推荐分组列表页（promote 类型「查看更多」目标页）：/promote_list 分页网格。
class PromoteShowPage extends StatefulWidget {
  const PromoteShowPage({super.key, required this.id, required this.title});

  final PlatformInt64 id;
  final String title;

  @override
  State<PromoteShowPage> createState() => _PromoteShowPageState();
}

class _PromoteShowPageState extends State<PromoteShowPage> {
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text(widget.title)),
      body: AlbumGrid(
        loadPage: (page) async {
          // promote page从0开始
          final info = await getPromoteList(id: widget.id, page: page - 1);
          // promote_list 与搜索结果结构一致，统一映射为 SearchInfo 复用分页网格
          return SearchInfo(
            searchQuery: '',
            total: info.total,
            content: info.list,
          );
        },
      ),
    );
  }
}
