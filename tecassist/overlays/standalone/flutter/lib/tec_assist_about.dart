import 'package:flutter/material.dart';
import 'models/platform_model.dart';

Future<void> showTecAssistAbout(BuildContext context) => showDialog<void>(
  context: context,
  builder: (dialogContext) => AlertDialog(
    title: const Text('Informazioni su Tec Assist'),
    content: SizedBox(width: 360, height: MediaQuery.of(dialogContext).size.height * 0.55, child: const TecAssistAboutContent()),
    actions: [TextButton(onPressed: () => Navigator.pop(dialogContext), child: const Text('Chiudi'))],
  ),
);

class TecAssistAboutContent extends StatelessWidget {
  const TecAssistAboutContent({super.key});
  @override
  Widget build(BuildContext context) => FutureBuilder<List<String>>(
    future: Future.wait([bind.mainGetVersion(), bind.mainGetLicense()]),
    builder: (context, snapshot) => SingleChildScrollView(child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Center(child: Image.asset('assets/tec_assist_logo.png', height: 90, fit: BoxFit.contain)),
      const SizedBox(height: 16),
      SelectableText('Tec Assist Standalone\nVersione ${snapshot.data?[0] ?? "..."} · 1.4.4.25 TEST\n© 2026 ideatore Luca Tecchio'),
      const SizedBox(height: 16),
      const Text('Informazioni e licenze', style: TextStyle(fontWeight: FontWeight.w600)),
      const SizedBox(height: 8),
      SelectableText('Basato su RustDesk.\nCopyright © 2026 Purslane Ltd.\n${snapshot.data?[1] ?? "Caricamento licenza..."}'),
      if (snapshot.hasError) const Text('Licenza completa inclusa nel pacchetto: LICENZA_RUSTDESK.txt.'),
    ])),
  );
}
