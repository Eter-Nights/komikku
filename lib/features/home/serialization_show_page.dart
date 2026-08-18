import 'package:flutter/material.dart';

import 'package:komikku/shared/widgets/album_grid.dart';
import 'package:komikku/shared/widgets/tag_bar.dart';
import 'package:komikku/rust/api.dart';
import 'package:komikku/rust/service/model.dart';

/// 每周连载更新页：第一行日期标签（周一~周日、已完结），第二行类型标签（全部/日漫/韩漫）。
/// 底层接口为 serialization（type: all/manga/hanman，date: 0~7），默认请求今天日期 + 全部。
class SerializationShowPage extends StatefulWidget {
  const SerializationShowPage({super.key});

  @override
  State<SerializationShowPage> createState() => _SerializationShowPageState();
}

class _SerializationShowPageState extends State<SerializationShowPage> {
  /// 日期标签：周一~周日对应 1-7，已完结对应 0
  static const _dateTabs = [
    (label: '周一', value: '1'),
    (label: '周二', value: '2'),
    (label: '周三', value: '3'),
    (label: '周四', value: '4'),
    (label: '周五', value: '5'),
    (label: '周六', value: '6'),
    (label: '周日', value: '7'),
    (label: '已完结', value: '0'),
  ];

  /// 类型标签：全部/日漫/韩漫
  static const _typeTabs = [
    (label: '全部', value: 'all'),
    (label: '日漫', value: 'manga'),
    (label: '韩漫', value: 'hanman'),
  ];

  final _gridKey = GlobalKey<AlbumGridState>();
  late String _date;
  late String _type;

  @override
  void initState() {
    super.initState();
    // 默认请求今天日期（DateTime.weekday: 1=周一 ... 7=周日）
    _date = '${DateTime.now().weekday}';
    _type = 'all';
  }

  void _onDateChanged(String value) {
    if (_date == value) return;
    setState(() => _date = value);
    _gridKey.currentState?.reset();
  }

  void _onTypeChanged(String value) {
    if (_type == value) return;
    setState(() => _type = value);
    _gridKey.currentState?.reset();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('每周连载更新')),
      body: Column(
        children: [
          TagBar<String>(
            items: _dateTabs,
            selected: _date,
            onSelected: _onDateChanged,
          ),
          TagBar<String>(
            items: _typeTabs,
            selected: _type,
            onSelected: _onTypeChanged,
          ),
          Expanded(
            child: AlbumGrid(
              key: _gridKey,
              // serialization 无 total 字段，标记 unknownTotal 按「本页非空」判断是否还有下一页
              unknownTotal: true,
              loadPage: (page) async {
                final info = await getSerialization(
                  date: _date,
                  serialType: _type,
                  page: page,
                );
                return SearchInfo(
                  searchQuery: '',
                  total: 0,
                  content: info.list,
                );
              },
            ),
          ),
        ],
      ),
    );
  }
}
