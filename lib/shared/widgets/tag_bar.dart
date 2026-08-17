import 'package:flutter/material.dart';

/// 可点击标签条：一组 ChoiceChip，选中高亮。
/// 支持横向滚动（默认）或换行（wrap=true）两种布局。
/// 只负责展示标签并回调选中值，不关心具体业务逻辑。
class TagBar<T> extends StatelessWidget {
  const TagBar({
    super.key,
    required this.items,
    required this.selected,
    required this.onSelected,
    this.wrap = false,
  });

  /// 标签项：label 显示文本，value 选中值
  final List<({String label, T value})> items;

  /// 当前选中的值
  final T selected;

  /// 选中回调（传入标签的 value）
  final ValueChanged<T> onSelected;

  /// true 时用 Wrap 换行布局（适合标签较多或需要多行场景），默认横向滚动
  final bool wrap;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;

    Widget chip(int i) {
      final item = items[i];
      final active = item.value == selected;
      return ChoiceChip(
        label: Text(item.label),
        selected: active,
        showCheckmark: false,
        onSelected: (_) => onSelected(item.value),
        labelStyle: TextStyle(
          fontSize: 13,
          color: active ? scheme.onPrimaryContainer : scheme.onSurfaceVariant,
          fontWeight: active ? FontWeight.w600 : FontWeight.normal,
        ),
        backgroundColor: scheme.surfaceContainerHigh,
        selectedColor: scheme.primaryContainer,
        side: BorderSide.none,
        visualDensity: VisualDensity.compact,
      );
    }

    if (wrap) {
      return Padding(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
        child: SizedBox(
          width: double.infinity,
          child: Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [for (var i = 0; i < items.length; i++) chip(i)],
          ),
        ),
      );
    }

    return SizedBox(
      height: 48,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        itemCount: items.length,
        separatorBuilder: (_, _) => const SizedBox(width: 8),
        itemBuilder: (context, i) => chip(i),
      ),
    );
  }
}
