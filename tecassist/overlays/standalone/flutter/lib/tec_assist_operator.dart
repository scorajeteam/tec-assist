import 'package:flutter/material.dart';
import 'models/platform_model.dart';

// Explicit per-launch operator selection; the saved label only prefills the form.
final tecAssistOperator = ValueNotifier<String>('');
Future<bool> tecAssistChooseOperator(BuildContext context) async {
  final previous = tecAssistOperator.value;
  final controller = TextEditingController(text: previous.isNotEmpty ? previous : bind.mainGetLocalOption(key: 'tec-assist-operator-name'));
  final form = GlobalKey<FormState>();
  final selected = await showDialog<String>(context: context, barrierDismissible: false, builder: (dialogContext) => AlertDialog(
    title: const Text('Operatore Tec Assist'),
    content: SizedBox(width: 320, child: Form(key: form, child: Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.start, children: [
      const Text('Indica chi sta usando la Console. Il cliente vedrà questo nome nelle nuove connessioni.'),
      const SizedBox(height: 16),
      TextFormField(controller: controller, autofocus: true, maxLength: 40, decoration: const InputDecoration(labelText: 'Nome operatore', hintText: 'Ad esempio Luca o Marco'),
        validator: (value) => (value ?? '').trim().isEmpty ? 'Inserisci il nome operatore' : null,
        onFieldSubmitted: (_) { if(form.currentState!.validate()) Navigator.pop(dialogContext, controller.text.trim()); }),
    ]))),
    actions: [
      TextButton(onPressed: () => Navigator.pop(dialogContext), child: const Text('Annulla')),
      ElevatedButton(onPressed: () { if(form.currentState!.validate()) Navigator.pop(dialogContext, controller.text.trim()); }, child: const Text('Conferma operatore')),
    ],
  ));
  if (selected == null || selected.trim().isEmpty) return false;
  await bind.mainSetLocalOption(key: 'tec-assist-operator-name', value: selected.trim());
  tecAssistOperator.value = selected.trim();
  return true;
}
Future<bool> tecAssistEnsureOperator(BuildContext context) async => tecAssistOperator.value.isNotEmpty || await tecAssistChooseOperator(context);
Widget tecAssistOperatorPanel() => ValueListenableBuilder<String>(valueListenable: tecAssistOperator, builder: (context, name, _) => Padding(
  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
  child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
    const Text('Operatore', style: TextStyle(fontSize: 12)),
    Text(name.isEmpty ? 'Da selezionare' : name, style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 16)),
    const SizedBox(height: 16),
    ElevatedButton.icon(
      style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF1565C0), foregroundColor: Colors.white, padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 14)),
      onPressed: () => tecAssistChooseOperator(context),
      icon: const Icon(Icons.person_outline, size: 18),
      label: Text(name.isEmpty ? 'Seleziona operatore' : 'Cambia operatore', style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600)),
    ),
  ]),
));
