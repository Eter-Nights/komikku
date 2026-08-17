import 'package:flutter/material.dart';

import 'package:komikku/features/bookshelf/bookshelf_page.dart';
import 'package:komikku/features/category/category_page.dart';
import 'package:komikku/features/home/home_page.dart';
import 'package:komikku/features/profile/profile_page.dart';

/// 应用根组件：主题 + 底部导航（首页 / 分类 / 书架 / 我的）
class KomikkuApp extends StatelessWidget {
  const KomikkuApp({super.key});

  @override
  Widget build(BuildContext context) {
    final scheme = ColorScheme.fromSeed(
      seedColor: Colors.deepPurple,
      brightness: Brightness.dark,
    );
    return MaterialApp(
      title: 'Komikku',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: scheme,
        scaffoldBackgroundColor: scheme.surface,
        navigationBarTheme: NavigationBarThemeData(
          backgroundColor: scheme.surfaceContainer,
          indicatorColor: scheme.primaryContainer,
        ),
      ),
      home: const HomeShell(),
    );
  }
}

/// 底部导航壳：4 个 Tab
class HomeShell extends StatefulWidget {
  const HomeShell({super.key});

  @override
  State<HomeShell> createState() => _HomeShellState();
}

class _HomeShellState extends State<HomeShell> {
  int _index = 0;

  static const _pages = <Widget>[
    HomePage(),
    CategoryPage(),
    BookshelfPage(),
    ProfilePage(),
  ];

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: IndexedStack(index: _index, children: _pages),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _index,
        onDestinationSelected: (i) => setState(() => _index = i),
        destinations: const [
          NavigationDestination(
            icon: Icon(Icons.home_outlined),
            selectedIcon: Icon(Icons.home),
            label: '首页',
          ),
          NavigationDestination(
            icon: Icon(Icons.category_outlined),
            selectedIcon: Icon(Icons.category),
            label: '分类',
          ),
          NavigationDestination(
            icon: Icon(Icons.collections_bookmark_outlined),
            selectedIcon: Icon(Icons.collections_bookmark),
            label: '书架',
          ),
          NavigationDestination(
            icon: Icon(Icons.person_outline),
            selectedIcon: Icon(Icons.person),
            label: '我的',
          ),
        ],
      ),
    );
  }
}
