import 'package:flutter/material.dart';

/// 栏目标题行：左侧标题，右侧「查看更多 ›」
class SectionHeader extends StatelessWidget {
  const SectionHeader({super.key, required this.title, this.onMore});

  final String title;
  final VoidCallback? onMore;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final titleStyle = theme.textTheme.titleMedium?.copyWith(
      fontWeight: FontWeight.bold,
      color: scheme.onSurface,
    );

    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 4, 8, 12),
      child: Row(
        children: [
          Expanded(
            child: Text(
              title,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: titleStyle,
            ),
          ),
          if (onMore != null)
            InkWell(
              onTap: onMore,
              borderRadius: BorderRadius.circular(8),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                child: Row(
                  children: [
                    Text(
                      '查看更多',
                      style: theme.textTheme.bodyMedium?.copyWith(
                        color: scheme.primary,
                      ),
                    ),
                    Icon(Icons.chevron_right, size: 18, color: scheme.primary),
                  ],
                ),
              ),
            ),
        ],
      ),
    );
  }
}
