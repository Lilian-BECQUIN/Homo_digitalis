# Pipeline d'assets 3D Réalistes (Phase 1b)

Ce pipeline intègre des modèles 3D anatomiques humains réalistes (Homme et Femme) segmentés à partir de données d'imagerie médicale (IRM et atlas anatomiques peer-reviewed).

## Sources scientifiques et licences
- **Homme (Adult Male)** : **BodyParts3D 4.0** (Database Center for Life Science, CC BY 4.0).
  - 2 234 pièces anatomiques segmentées, 2,28 millions de triangles.
  - Couverture complète : squelette osseux, colonne vertébrale (rachis C1-L5 + sacrum), système musculaire, arbre artériel, réseau veineux de retour, système nerveux central et périphérique, organes internes (viscères), enveloppe cutanée complète.
- **Femme (Adult Female)** : **Human Reference Atlas (HuBMAP) v1.10** (Browne & Schlehlein, CC BY 4.0).
  - 1 220 structures segmentées, 2,76 millions de triangles.
  - Couverture complète adaptée : squelette, colonne vertébrale, musculature, cerveau subdivisé, viscères, système vasculaire, enveloppe cutanée féminine.

## Script automatisé : `tools/build_anatomy.py`
Le script télécharge et assemble les maillages binaires en deux fichiers glTF 2.0 binaires optimisés :
- `crates/homo-app/assets/models/human_male.glb`
- `crates/homo-app/assets/models/human_female.glb`

Chaque fichier contient 9 nœuds majeurs standardisés correspondant aux composants Bevy :
1. **Skin** (`AnatomyLayer::Skin`) : enveloppe cutanée extérieure (corps complet).
2. **Vertebrae** (`AnatomyLayer::Vertebrae`) : rachis complet (7 cervicales, 12 thoraciques, 5 lombaires, sacrum et coccyx).
3. **Skeleton** (`AnatomyLayer::Skeleton`) : crâne, cage thoracique (sternum, côtes I-XII), ceintures et membres osseux.
4. **Muscles** (`AnatomyLayer::Muscle`) : ensemble des muscles squelettiques (pectoraux, deltoïdes, biceps, abdominaux, fessiers, quadriceps...).
5. **Heart** (`AnatomyLayer::Heart`) : myocarde, oreillettes, ventricules (animé avec la simulation Suga/élastance).
6. **Arteries** (`AnatomyLayer::Arteries`) : arbre artériel pulsatile (aorte, carotides, fémorales...).
7. **Veins** (`AnatomyLayer::Veins`) : réseau veineux complet (veines caves supérieure/inférieure, jugulaires, rénales, fémorales...).
8. **Nervous** (`AnatomyLayer::Nervous`) : encéphale, cervelet, moelle épinière et troncs nerveux périphériques.
9. **Viscera** (`AnatomyLayer::Viscera`) : poumons, foie, estomac, intestins, reins, vessie.

## Modes de visualisation et d'isolation (Solo)
- **Sélecteur Homme / Femme** : bascule instantanée entre les anatomies masculine et féminine.
- **Boutons Solo** :
  - *Squelette complet*
  - *Vertèbres seules*
  - *Muscles seuls*
  - *Système vasculaire complet (Artères + Veines + Cœur)*
  - *Toutes les veines seules*
  - *Système nerveux seul*
  - *Organes internes seuls*
  - *Corps complet (Peau opaque)*
  - *Corps complet (Écorché translucide)*
- **Curseurs d'opacité** : contrôle continu 0 % à 100 % sur chacun des 9 calques.
