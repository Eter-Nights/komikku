import 'package:flutter/material.dart';

import 'package:komikku/shared/widgets/album_card.dart';
import 'package:komikku/rust/service/model.dart';

/// 横向滚动的漫画卡片行（首页栏目内容）。
/// 卡片宽度固定，露出半张提示可继续滑动；左缘渐隐遮罩。
class AlbumRow extends StatelessWidget {
  const AlbumRow({
    super.key,
    required this.items,
    this.cardWidth = 120,
    this.onTap,
  });

  final List<AlbumBriefInfo> items;
  final double cardWidth;
  final void Function(AlbumBriefInfo)? onTap;

  @override
  Widget build(BuildContext context) {
    if (items.isEmpty) {
      return const SizedBox(height: 120, child: Center(child: Text('暂无内容')));
    }

    return SizedBox(
      height: cardHeight,
      child: Stack(
        children: [
          ListView.separated(
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.symmetric(horizontal: 16),
            itemCount: items.length,
            separatorBuilder: (_, _) => const SizedBox(width: 12),
            itemBuilder: (context, i) {
              final album = items[i];
              return SizedBox(
                width: cardWidth,
                child: AlbumCard(
                  album: album,
                  onTap: onTap == null ? null : () => onTap!(album),
                ),
              );
            },
          ),
          // 左缘渐隐遮罩
          IgnorePointer(
            child: Container(
              width: 24,
              decoration: BoxDecoration(
                gradient: LinearGradient(
                  colors: [
                    Theme.of(context).scaffoldBackgroundColor,
                    Colors.transparent,
                  ],
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  /// 卡片高度 = 封面(3:4) + 文本区(标题2行 + 作者 + 间距)
  double get cardHeight => cardWidth * 4 / 3 + 64;
}
