use spru::item::IdT;
use spru_util::cloned;
use tagset::tagset;

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru_script::script(include = [Impl])]
struct S {
    #[get]
    #[set]
    a: i64,

    #[get]
    #[set]
    b: Option<IdT<S>>,
}

#[spru_script::script(partial = Impl)]
impl S {
    #[create]
    fn create(val: i64, _dummy: i64) -> cloned::Create<S> {
        cloned::create(Self {
            a: val,
            b: None,
        })
    }

    #[function]
    fn f() -> i64 {
        9i64
    }
}

#[tagset(impl tagset::proxy::serde::Serialize)]
#[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
#[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
#[tagset(impl spru::State)]
#[tagset(derive(Debug))]
#[tagset(S)]
struct MyState;

#[tagset(impl spru::Action {
    type State = MyState;
})]
#[tagset(impl tagset::proxy::serde::Serialize)]
#[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
#[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
#[tagset(derive(Debug, Clone))]
#[tagset(include(cloned::Actions<S>))]
struct MyAction;

struct MyLexicon;

impl spru_script::StatelessLexicon for MyLexicon {
    type Language = spru_script_rhai::Rhai;

    fn register_stateless(registration: &mut spru_script_rhai::Registration1<'_>) {
        
    }
}

impl spru_script::Lexicon for MyLexicon {
    
    type Action = MyAction;

    fn register_state<Storage>(registration: &mut spru_script_rhai::Registration1<'_>)
    where
        Storage: spru::item::Storage<State = MyState>,
    {
        spru_script_rhai::rhai! { <Storage, MyAction> registration {
            register_S => S as S;
        } };
        // register_S!(<Storage, MyAction> registration => S);
    }
}

#[test]
fn script() {
    use spru_script::LanguageExec as _;
    let storage = spru_util::storage::Standalone::<MyState>::new();
    let rhai = spru_script_rhai::RhaiInstance::<MyLexicon>::default();
    let mut test_interactor = spru::interactor::test_util::TestInteractor::new(storage);
    let mut interactor = test_interactor.interactor::<MyAction, _, ()>(&());
    let s = interactor
        .create(cloned::create(S { a: 3i64, b: None, }));
    let root = s.id();

    interactor.flush().unwrap();

    let mut interactor = test_interactor.interactor::<MyAction, _, ()>(&root);

    let script = r#"
    let b = type.S.create(7, 5);
    context.root.b = b;
    context.root.b.a
    "#;

    let a: i64 = rhai.exec(&mut interactor, script, rhai::Dynamic::UNIT)
        .unwrap();

    assert_eq!(a, 7i64);
}

#[test]
fn eval() {
    use spru_script::{LanguageExec as _, LanguageEval as _};
    let storage = spru_util::storage::Standalone::<MyState>::new();
    let rhai = spru_script_rhai::RhaiInstance::<MyLexicon>::default();
    let mut test_interactor = spru::interactor::test_util::TestInteractor::new(storage);
    let mut interactor = test_interactor.interactor::<MyAction, _, ()>(&());
    let s = interactor
        .create(cloned::create(S { a: 3i64, b: None, }));
    let root = s.id();

    interactor.flush().unwrap();

    let mut interactor = test_interactor.interactor::<MyAction, _, ()>(&root);

    let exec_script = r#"
    let b = type.S.create(7, 5);
    context.root.b = b;
    context.root.b.a
    "#;

    let a: i64 = rhai.exec(&mut interactor, exec_script, rhai::Dynamic::UNIT)
        .unwrap();

    let eval_script = r#"
    context.root.b.a + args
    "#;

    let addend = 3i64;

    let b: i64 = rhai.eval(interactor.ledger().storage(), &root, eval_script, rhai::Dynamic::from(addend))
        .unwrap();

    assert_eq!(a, 7i64);
    assert_eq!(a + addend, b);

    let bad_eval_script = r#"
    let b = type.S.create(7, 5);
    context.root.b = b;
    context.root.b.a
    "#;

    let c: Result<i64, _> = rhai.eval(interactor.ledger().storage(), &root, bad_eval_script, ());

    assert!(c.is_err());
}
