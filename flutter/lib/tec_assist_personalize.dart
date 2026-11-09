import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'models/platform_model.dart';

class TecConsoleStyle {
  final Color background;
  final Color accent;
  final Uint8List? logo;
  const TecConsoleStyle({this.background = const Color(0xFF1565C0),
    this.accent = const Color(0xFFF4C20D), this.logo});
  Color get foreground => background.computeLuminance() > 0.4 ? Colors.black87 : Colors.white;
  Map<String, dynamic> toJson() => {'background': background.value,
    'accent': accent.value, if (logo != null) 'logo': base64Encode(logo!)};
}

final tecConsoleStyle = ValueNotifier<TecConsoleStyle>(const TecConsoleStyle());
bool _loaded = false;
void loadTecConsoleStyle() {
  if (_loaded) return;
  _loaded = true;
  try {
    final raw = bind.mainGetLocalOption(key: 'tec-assist-console-style-v1');
    if (raw.isEmpty) return;
    final value = jsonDecode(raw) as Map<String, dynamic>;
    tecConsoleStyle.value = TecConsoleStyle(
      background: Color(value['background'] as int), accent: Color(value['accent'] as int),
      logo: value['logo'] == null ? null : base64Decode(value['logo'] as String));
  } catch (_) {
    // A malformed preference must never prevent the technician from connecting.
    tecConsoleStyle.value = const TecConsoleStyle();
  }
}

Widget tecConsoleBrandCard(TecConsoleStyle style, {bool preview = false}) => Container(
  margin: EdgeInsets.all(preview ? 0 : 16),
  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 20),
  decoration: BoxDecoration(color: style.background,
    borderRadius: BorderRadius.circular(18),
    boxShadow: [BoxShadow(color: style.background.withOpacity(0.18), blurRadius: 14, offset: const Offset(0, 5))]),
  child: Column(mainAxisSize: MainAxisSize.min, children: [
    if (style.logo == null) Image.asset('assets/tec_assist_logo.png', height: 125, fit: BoxFit.contain)
    else Image.memory(style.logo!, height: 125, fit: BoxFit.contain,
      errorBuilder: (_, __, ___) => Image.asset('assets/tec_assist_logo.png', height: 125)),
    const SizedBox(height: 14),
    Text('Console', style: TextStyle(color: style.accent, fontSize: 22, fontWeight: FontWeight.w700)),
    const SizedBox(height: 5),
    Text('Assistenza remota', style: TextStyle(color: style.foreground, fontSize: 12)),
  ]),
);

Widget tecConsolePersonalHeader() {
  loadTecConsoleStyle();
  return ValueListenableBuilder<TecConsoleStyle>(valueListenable: tecConsoleStyle,
    builder: (_, style, __) => tecConsoleBrandCard(style));
}

String _hex(Color value) => '#${(value.value & 0xFFFFFF).toRadixString(16).padLeft(6, '0').toUpperCase()}';
Color? _parse(String value) => RegExp(r'^#[0-9a-fA-F]{6}$').hasMatch(value.trim())
  ? Color(int.parse('FF${value.trim().substring(1)}', radix: 16)) : null;

Future<void> showTecConsolePersonalize(BuildContext context) async {
  loadTecConsoleStyle();
  final saved = await showDialog<TecConsoleStyle>(context: context,
    builder: (_) => const _TecConsolePersonalizeDialog());
  if (saved == null) return;
  try {
    await bind.mainSetLocalOption(key: 'tec-assist-console-style-v1', value: jsonEncode(saved.toJson()));
    tecConsoleStyle.value = saved;
  } catch (_) {
    if (context.mounted) ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Preferenze non salvate. Riprova.')));
  }
}

class _TecConsolePersonalizeDialog extends StatefulWidget {
  const _TecConsolePersonalizeDialog();
  @override
  State<_TecConsolePersonalizeDialog> createState() => _TecConsolePersonalizeDialogState();
}

class _TecConsolePersonalizeDialogState extends State<_TecConsolePersonalizeDialog> {
  late final TextEditingController background;
  late final TextEditingController accent;
  Uint8List? logo;
  String? error;
  bool picking = false;
  @override
  void initState() {
    super.initState();
    final current = tecConsoleStyle.value;
    background = TextEditingController(text: _hex(current.background));
    accent = TextEditingController(text: _hex(current.accent));
    logo = current.logo;
  }
  @override
  void dispose() { background.dispose(); accent.dispose(); super.dispose(); }
  TecConsoleStyle get draft => TecConsoleStyle(
    background: _parse(background.text) ?? tecConsoleStyle.value.background,
    accent: _parse(accent.text) ?? tecConsoleStyle.value.accent, logo: logo);

  Future<void> chooseLogo() async {
    setState(() { picking = true; error = null; });
    try {
      final result = await FilePicker.platform.pickFiles(type: FileType.custom,
        allowedExtensions: ['png', 'jpg', 'jpeg'], allowMultiple: false);
      if (result == null) return;
      final path = result.files.single.path;
      if (path == null) throw const FormatException('File non disponibile.');
      final file = File(path);
      if (await file.length() > 1024 * 1024) throw const FormatException('Scegli un logo di massimo 1 MB.');
      final bytes = await file.readAsBytes();
      final codec = await ui.instantiateImageCodec(bytes, targetWidth: 512);
      try {
        final frame = await codec.getNextFrame();
        final normalized = await frame.image.toByteData(format: ui.ImageByteFormat.png);
        frame.image.dispose();
        if (normalized == null || normalized.lengthInBytes > 1024 * 1024) throw const FormatException('Logo troppo grande.');
        if (mounted) setState(() => logo = normalized.buffer.asUint8List());
      } finally { codec.dispose(); }
    } catch (e) {
      if (mounted) setState(() => error = e is FormatException ? e.message : 'Immagine non valida. Usa un file PNG o JPG.');
    } finally {
      if (mounted) setState(() => picking = false);
    }
  }

  Future<void> chooseColor(String label, TextEditingController controller) async {
    var selected = _parse(controller.text) ?? const Color(0xFF1565C0);
    const swatches = <String, Color>{
      'Blu Tec Assist': Color(0xFF1565C0), 'Giallo Tec Assist': Color(0xFFF4C20D),
      'Blu notte': Color(0xFF102A43), 'Azzurro': Color(0xFF039BE5),
      'Turchese': Color(0xFF00897B), 'Verde': Color(0xFF2E7D32),
      'Viola': Color(0xFF6A1B9A), 'Rosa': Color(0xFFC2185B),
      'Rosso': Color(0xFFC62828), 'Arancione': Color(0xFFEF6C00),
      'Bianco': Color(0xFFFFFFFF), 'Grigio chiaro': Color(0xFFECEFF1),
      'Grigio': Color(0xFF607D8B), 'Grigio scuro': Color(0xFF37474F),
      'Nero': Color(0xFF111111),
    };
    final result = await showDialog<Color>(context: context, builder: (pickerContext) =>
      StatefulBuilder(builder: (pickerContext, update) {
        Widget channel(String name, int value, Color color, Color Function(int) change) => Row(children: [
          SizedBox(width: 54, child: Text(name)),
          Expanded(child: Slider(value: value.toDouble(), min: 0, max: 255, divisions: 255,
            activeColor: color, label: '$value', semanticFormatterCallback: (v) => '$name ${v.round()}',
            onChanged: (v) => update(() => selected = change(v.round())))),
        ]);
        return AlertDialog(
          title: Text(label),
          content: SizedBox(width: 380, child: SingleChildScrollView(child: Column(
            mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.stretch, children: [
              Container(height: 58, decoration: BoxDecoration(color: selected,
                borderRadius: BorderRadius.circular(12), border: Border.all(color: Colors.grey))),
              const SizedBox(height: 18),
              const Text('Scegli dalla tavolozza'),
              const SizedBox(height: 10),
              Wrap(spacing: 8, runSpacing: 8, children: swatches.entries.map((entry) =>
                Tooltip(message: entry.key, child: Semantics(label: entry.key, button: true,
                  selected: selected.value == entry.value.value, child: InkWell(
                    onTap: () => update(() => selected = entry.value), borderRadius: BorderRadius.circular(10),
                    child: Container(width: 46, height: 42, decoration: BoxDecoration(color: entry.value,
                      borderRadius: BorderRadius.circular(10), border: Border.all(
                        color: selected.value == entry.value.value ? Theme.of(context).colorScheme.primary : Colors.grey,
                        width: selected.value == entry.value.value ? 3 : 1)),
                      child: selected.value == entry.value.value ? Icon(Icons.check,
                        color: entry.value.computeLuminance() > 0.4 ? Colors.black : Colors.white) : null)))))
                .toList()),
              const SizedBox(height: 20),
              const Text('Oppure regola la tonalità'),
              channel('Rosso', selected.red, Colors.red, (v) => Color.fromARGB(255, v, selected.green, selected.blue)),
              channel('Verde', selected.green, Colors.green, (v) => Color.fromARGB(255, selected.red, v, selected.blue)),
              channel('Blu', selected.blue, Colors.blue, (v) => Color.fromARGB(255, selected.red, selected.green, v)),
            ]))),
          actions: [TextButton(onPressed: () => Navigator.pop(pickerContext), child: const Text('Annulla')),
            ElevatedButton(onPressed: () => Navigator.pop(pickerContext, selected), child: const Text('Usa colore'))],
        );
      }));
    if (result != null && mounted) setState(() { controller.text = _hex(result); error = null; });
  }

  Widget colorField(String label, TextEditingController controller) => Padding(
    padding: const EdgeInsets.only(bottom: 12),
    child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
      OutlinedButton.icon(onPressed: () => chooseColor(label, controller),
        icon: Icon(Icons.palette, color: _parse(controller.text) ?? Colors.grey),
        label: Text('Scegli $label')),
      TextField(controller: controller, maxLength: 7,
        decoration: const InputDecoration(labelText: 'Codice colore (facoltativo)', counterText: ''),
        onChanged: (_) => setState(() => error = null)),
    ]));

  void save() {
    if (_parse(background.text) == null || _parse(accent.text) == null) {
      setState(() => error = 'Inserisci i colori nel formato #RRGGBB, ad esempio #1565C0.'); return;
    }
    final current = draft;
    final a = current.background.computeLuminance();
    final b = current.accent.computeLuminance();
    final contrast = ((a > b ? a : b) + 0.05) / ((a > b ? b : a) + 0.05);
    if (contrast < 3) { setState(() => error = 'I colori sono troppo simili: scegli un testo più leggibile sullo sfondo.'); return; }
    Navigator.pop(context, current);
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('Personalizza Console'),
    content: SizedBox(width: 560, height: MediaQuery.of(context).size.height * 0.64,
      child: SingleChildScrollView(child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
        const Text('Logo e colori del riquadro Console. Le modifiche si applicano solo su questo PC.'),
        const SizedBox(height: 16),
        Center(child: SizedBox(width: 230, child: tecConsoleBrandCard(draft, preview: true))),
        const SizedBox(height: 20),
        OutlinedButton.icon(onPressed: picking ? null : chooseLogo,
          icon: const Icon(Icons.add_photo_alternate_outlined), label: Text(picking ? 'Caricamento...' : 'Scegli logo PNG o JPG')),
        const Text('Massimo 1 MB. Il logo viene salvato nelle preferenze: puoi spostare il file originale.', style: TextStyle(fontSize: 12)),
        const SizedBox(height: 14),
        colorField('Colore sfondo del logo', background),
        colorField('Colore scritta Console', accent),
        if (error != null) Padding(padding: const EdgeInsets.only(top: 8),
          child: Text(error!, style: TextStyle(color: Theme.of(context).colorScheme.error))),
      ]))),
    actions: [
      TextButton(onPressed: picking ? null : () => setState(() {
        background.text = '#1565C0'; accent.text = '#F4C20D'; logo = null; error = null;
      }), child: const Text('Ripristina Tec Assist')),
      TextButton(onPressed: () => Navigator.pop(context), child: const Text('Annulla')),
      ElevatedButton(onPressed: picking ? null : save, child: const Text('Salva')),
    ],
  );
}
