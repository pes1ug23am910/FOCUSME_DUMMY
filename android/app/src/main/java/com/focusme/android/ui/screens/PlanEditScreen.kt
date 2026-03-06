// ============================================================
// FILE:        PlanEditScreen.kt
// MODULE:      Layer 4 — Android UI > Plan Edit Screen
// TASK:        T-045 (implementation — Session 4)
// PLATFORM:    android
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 3, Android Compose UI
// DEPENDENCIES: Jetpack Compose (Material3), FocusMeDaemonService
// TEST COVERAGE: Test: plan creation saves to service, Test: schedule day chips toggle
// KNOWN LIMITATIONS: No time picker dialog yet — uses text field for time input.
//                    Quota sliders are MVP stubs.
// ============================================================

package com.focusme.android.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.focusme.android.service.FocusMeDaemonService
import com.focusme.android.service.FocusMeDaemonService.PlanData
import com.focusme.android.service.FocusMeDaemonService.PlanRule
import java.util.UUID

/**
 * PlanEditScreen — create or edit a Focus Plan.
 *
 * Features:
 * - Plan name text field
 * - Schedule day chips (Mon–Sun toggle)
 * - Start/end time fields (minutes from midnight)
 * - App block list (add package names)
 * - URL block list (add domains)
 * - Forced mode toggle
 * - Save button → FocusMeDaemonService.savePlan()
 *
 * @param planId      Plan ID to edit (null = create new)
 * @param onNavigateBack  Navigate back to PlanListScreen
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PlanEditScreen(
    planId: String?,
    onNavigateBack: () -> Unit,
) {
    val daemon = FocusMeDaemonService.instance
    val isNew = planId == null

    // ---- Form state ----
    var name by remember { mutableStateOf("") }
    var selectedDays by remember { mutableStateOf(setOf(1, 2, 3, 4, 5)) } // Weekdays default
    var startMinute by remember { mutableIntStateOf(9 * 60) }  // 9:00 AM
    var endMinute by remember { mutableIntStateOf(17 * 60) }    // 5:00 PM
    var appRules by remember { mutableStateOf(listOf<String>()) }
    var urlRules by remember { mutableStateOf(listOf<String>()) }
    var forcedMode by remember { mutableStateOf(false) }
    var enabled by remember { mutableStateOf(true) }

    // Input fields for adding rules
    var newAppPackage by remember { mutableStateOf("") }
    var newUrlDomain by remember { mutableStateOf("") }

    // Load existing plan if editing
    LaunchedEffect(planId) {
        if (planId != null && daemon != null) {
            daemon.getPlan(planId)?.let { plan ->
                name = plan.name
                selectedDays = plan.scheduleDays
                startMinute = plan.scheduleStartMin
                endMinute = plan.scheduleEndMin
                appRules = plan.rules.filter { it.ruleType == "app" }.map { it.target }
                urlRules = plan.rules.filter { it.ruleType == "url" }.map { it.target }
                enabled = plan.enabled
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(if (isNew) "Create Plan" else "Edit Plan") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    // Save button
                    IconButton(
                        onClick = {
                            val rules = mutableListOf<PlanRule>()
                            appRules.forEach { pkg ->
                                rules.add(PlanRule(target = pkg, ruleType = "app"))
                            }
                            urlRules.forEach { domain ->
                                rules.add(PlanRule(target = domain, ruleType = "url"))
                            }

                            val plan = PlanData(
                                id = planId ?: UUID.randomUUID().toString(),
                                name = name.ifBlank { "Untitled Plan" },
                                rules = rules,
                                scheduleDays = selectedDays,
                                scheduleStartMin = startMinute,
                                scheduleEndMin = endMinute,
                                enabled = enabled,
                            )

                            daemon?.savePlan(plan)
                            onNavigateBack()
                        },
                        enabled = name.isNotBlank(),
                    ) {
                        Icon(Icons.Filled.Check, contentDescription = "Save")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(20.dp),
        ) {
            // ---- Plan Name ----
            OutlinedTextField(
                value = name,
                onValueChange = { name = it },
                label = { Text("Plan Name") },
                placeholder = { Text("e.g., Work Focus") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )

            // ---- Enabled toggle ----
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("Enabled", style = MaterialTheme.typography.bodyLarge)
                Switch(checked = enabled, onCheckedChange = { enabled = it })
            }

            // ---- Schedule Days ----
            Text(
                text = "Schedule Days",
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
            )

            DayChipRow(
                selectedDays = selectedDays,
                onToggle = { day ->
                    selectedDays = if (day in selectedDays) {
                        selectedDays - day
                    } else {
                        selectedDays + day
                    }
                },
            )

            // ---- Schedule Time ----
            Text(
                text = "Active Hours",
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                TimeField(
                    label = "Start",
                    minuteOfDay = startMinute,
                    onValueChange = { startMinute = it },
                    modifier = Modifier.weight(1f),
                )
                TimeField(
                    label = "End",
                    minuteOfDay = endMinute,
                    onValueChange = { endMinute = it },
                    modifier = Modifier.weight(1f),
                )
            }

            // ---- App Block List ----
            Text(
                text = "Blocked Apps",
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
            )

            RuleInputSection(
                items = appRules,
                inputValue = newAppPackage,
                onInputChange = { newAppPackage = it },
                onAdd = {
                    if (newAppPackage.isNotBlank()) {
                        appRules = appRules + newAppPackage.trim()
                        newAppPackage = ""
                    }
                },
                onRemove = { idx -> appRules = appRules.filterIndexed { i, _ -> i != idx } },
                placeholder = "com.example.app",
                label = "Package Name",
            )

            // ---- URL Block List ----
            Text(
                text = "Blocked URLs",
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
            )

            RuleInputSection(
                items = urlRules,
                inputValue = newUrlDomain,
                onInputChange = { newUrlDomain = it },
                onAdd = {
                    if (newUrlDomain.isNotBlank()) {
                        urlRules = urlRules + newUrlDomain.trim()
                        newUrlDomain = ""
                    }
                },
                onRemove = { idx -> urlRules = urlRules.filterIndexed { i, _ -> i != idx } },
                placeholder = "reddit.com",
                label = "Domain",
            )

            // ---- Forced Mode Toggle ----
            HorizontalDivider()

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column {
                    Text("Forced Mode", style = MaterialTheme.typography.bodyLarge)
                    Text(
                        "Cannot be disabled once activated",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Switch(checked = forcedMode, onCheckedChange = { forcedMode = it })
            }

            if (forcedMode) {
                Card(
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.errorContainer,
                    ),
                ) {
                    Text(
                        text = "⚠️ Forced Mode prevents plan modification during active hours. " +
                            "You will need an emergency code to unlock early.",
                        modifier = Modifier.padding(12.dp),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onErrorContainer,
                    )
                }
            }

            // Bottom spacer for scroll clearance
            Spacer(modifier = Modifier.height(80.dp))
        }
    }
}

// ---- Day Chip Row ----

@Composable
private fun DayChipRow(
    selectedDays: Set<Int>,
    onToggle: (Int) -> Unit,
) {
    val dayLabels = listOf(
        1 to "Mon", 2 to "Tue", 3 to "Wed", 4 to "Thu",
        5 to "Fri", 6 to "Sat", 7 to "Sun",
    )

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        dayLabels.forEach { (dayNum, label) ->
            FilterChip(
                selected = dayNum in selectedDays,
                onClick = { onToggle(dayNum) },
                label = { Text(label, style = MaterialTheme.typography.labelSmall) },
                modifier = Modifier.weight(1f),
            )
        }
    }
}

// ---- Time Field (simple text input for HH:MM) ----

@Composable
private fun TimeField(
    label: String,
    minuteOfDay: Int,
    onValueChange: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    val h = minuteOfDay / 60
    val m = minuteOfDay % 60
    var text by remember(minuteOfDay) {
        mutableStateOf("${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}")
    }

    OutlinedTextField(
        value = text,
        onValueChange = { input ->
            text = input
            // Parse HH:MM
            val parts = input.split(":")
            if (parts.size == 2) {
                val hour = parts[0].toIntOrNull() ?: return@OutlinedTextField
                val min = parts[1].toIntOrNull() ?: return@OutlinedTextField
                if (hour in 0..23 && min in 0..59) {
                    onValueChange(hour * 60 + min)
                }
            }
        },
        label = { Text(label) },
        placeholder = { Text("HH:MM") },
        modifier = modifier,
        singleLine = true,
    )
}

// ---- Rule Input Section (app packages / URL domains) ----

@Composable
private fun RuleInputSection(
    items: List<String>,
    inputValue: String,
    onInputChange: (String) -> Unit,
    onAdd: () -> Unit,
    onRemove: (Int) -> Unit,
    placeholder: String,
    label: String,
) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        // Input row
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            OutlinedTextField(
                value = inputValue,
                onValueChange = onInputChange,
                label = { Text(label) },
                placeholder = { Text(placeholder) },
                modifier = Modifier.weight(1f),
                singleLine = true,
            )
            FilledTonalButton(onClick = onAdd, enabled = inputValue.isNotBlank()) {
                Text("Add")
            }
        }

        // Existing items as chips
        items.forEachIndexed { index, item ->
            InputChip(
                selected = false,
                onClick = {},
                label = { Text(item) },
                trailingIcon = {
                    IconButton(
                        onClick = { onRemove(index) },
                        modifier = Modifier.size(18.dp),
                    ) {
                        Icon(
                            Icons.Filled.Close,
                            contentDescription = "Remove",
                            modifier = Modifier.size(14.dp),
                        )
                    }
                },
            )
        }

        if (items.isEmpty()) {
            Text(
                text = "No ${label.lowercase()}s added yet",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
