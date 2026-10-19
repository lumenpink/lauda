use lauda_types::calendar::{
    CalendarIntentParser, CalendarReferenceTime, Weekday, format_intent_date_chip,
    format_intent_datetime_chip, format_intent_time_chip,
};

fn sample_reference_time() -> CalendarReferenceTime {
    CalendarReferenceTime {
        year: 2026,
        month: 10,
        day: 19,
        hour: 10,
        minute: 0,
        weekday: Weekday::Monday,
        timestamp_utc: 1792404000,
    }
}

#[test]
fn test_calendar_intent_date_and_time_chips_full_data() {
    let ref_time = sample_reference_time();

    // 1. Evento com horário no dia seguinte ("amanhã às 12h30")
    let parsed_with_time =
        CalendarIntentParser::parse("Almoço com Marcelo amanhã às 12h30", &ref_time)
            .expect("parse com horário");

    assert_eq!(
        format_intent_date_chip(&parsed_with_time),
        "20/10/2026",
        "A data de processamento deve conter dia, mês e ano completos"
    );
    assert_eq!(
        format_intent_time_chip(&parsed_with_time),
        "12:30 - 13:30",
        "O horário processado deve conter o intervalo de início e fim"
    );
    assert_eq!(
        format_intent_datetime_chip(&parsed_with_time),
        "20/10/2026 · 12:30 - 13:30"
    );

    // 2. Evento de dia inteiro ("Aniversário amanhã")
    let parsed_all_day =
        CalendarIntentParser::parse("Aniversário amanhã", &ref_time).expect("parse dia inteiro");

    assert_eq!(format_intent_date_chip(&parsed_all_day), "20/10/2026");
    assert_eq!(format_intent_time_chip(&parsed_all_day), "Dia inteiro");
    assert_eq!(
        format_intent_datetime_chip(&parsed_all_day),
        "20/10/2026 (Dia inteiro)"
    );

    // 3. Evento overnight que cruza a meia-noite ("Plantão das 19h às 07h")
    let parsed_overnight =
        CalendarIntentParser::parse("Plantão no Hospital das 19h às 07h", &ref_time)
            .expect("parse overnight");

    assert_eq!(
        format_intent_date_chip(&parsed_overnight),
        "19/10/2026 → 20/10/2026",
        "Evento que cruza dias deve indicar as datas completas processadas"
    );
    assert_eq!(
        format_intent_time_chip(&parsed_overnight),
        "19:00 → 07:00",
        "Evento overnight deve indicar os horários de início e término"
    );
    assert_eq!(
        format_intent_datetime_chip(&parsed_overnight),
        "19/10/2026 19:00 → 20/10/2026 07:00"
    );
}

#[test]
fn test_mutation_detection_on_omitted_processed_date() {
    let ref_time = sample_reference_time();
    let parsed = CalendarIntentParser::parse("Reunião sexta às 15h", &ref_time).expect("parse");

    let date_chip = format_intent_date_chip(&parsed);
    let time_chip = format_intent_time_chip(&parsed);

    // Mutante 1: Omissão de ano (ex.: apenas "23/10")
    assert!(
        date_chip.contains("2026"),
        "O chip deve incluir o ano calculado para eliminar ambiguidade"
    );

    // Mutante 2: Retorno de texto genérico estático ("Com horário definido")
    assert_ne!(
        time_chip, "Com horário definido",
        "O chip não pode omitir os horários processados de início e fim"
    );
    assert!(
        time_chip.contains("15:00"),
        "O chip deve conter o horário de início"
    );
    assert!(
        time_chip.contains("16:00"),
        "O chip deve conter o horário de fim"
    );
}
