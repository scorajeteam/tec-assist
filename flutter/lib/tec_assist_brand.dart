import 'package:flutter/material.dart';
const tecAssistMode = String.fromEnvironment('TEC_ASSIST_MODE', defaultValue: 'console');
const tecAssistStandalone = tecAssistMode == 'standalone';
const tecAssistWindowTitle = tecAssistStandalone ? 'Standalone' : 'Console';
String tecAssistShortName(String name) {
  final clean = name.replaceAll(RegExp(r'[\x00-\x1F\x7F]'), ' ').trim();
  final label = clean.split('—').first.trim();
  return label.isEmpty ? 'Tecnico' : label;
}
String tecAssistConnectedLabel(List<String> names) {
  final labels = names.map(tecAssistShortName).toSet().toList();
  if (labels.isEmpty) return '';
  return '${labels.join(', ')} ${labels.length == 1 ? "si è collegato" : "si sono collegati"}';
}
Widget tecAssistBrandHeader({List<String> connectedNames = const []}) {
  return Container(
    margin: const EdgeInsets.fromLTRB(16, 16, 16, 20),
    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 16),
    decoration: BoxDecoration(color: const Color(0xFF1565C0), borderRadius: BorderRadius.circular(14), border: Border.all(color: const Color(0xFF33577C))),
    child: Column(mainAxisSize: MainAxisSize.min, children: [
      Image.asset('assets/tec_assist_logo.png', height: tecAssistStandalone ? 105 : 125, fit: BoxFit.contain),
      const SizedBox(height: 10),
      Row(mainAxisAlignment: MainAxisAlignment.center, children: [
        const Text(tecAssistWindowTitle, style: TextStyle(color: Color(0xFFF4C20D), fontSize: 20, fontWeight: FontWeight.w600)),
        if (tecAssistStandalone && connectedNames.isNotEmpty) ...[
          const SizedBox(width: 10),
          Flexible(child: Text(tecAssistConnectedLabel(connectedNames), textAlign: TextAlign.left, style: const TextStyle(color: Colors.white, fontSize: 12), maxLines: 2, overflow: TextOverflow.ellipsis)),
        ],
      ]),
      const SizedBox(height: 4),
      const Text('Assistenza remota', style: TextStyle(color: Color(0xFFDAE5F0), fontSize: 12)),
    ]),
  );
}